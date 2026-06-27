---
id: "ADR-0052"
date: "2026-06-27T12:23:42+00:00"
author: Toby Clemson
status: proposed
tags: [architecture, filesystem, message-bus, knowledge-corpus, foundations]
type: adr
title: "ADR-0052: Filesystem as Message Bus and Knowledge Corpus"
schema_version: 1
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: Toby Clemson
relates_to: ["adr:ADR-0051", "adr:ADR-0045", "adr:ADR-0001"]
supersedes: ["adr:ADR-0027"]
---

# ADR-0052: Filesystem as Message Bus and Knowledge Corpus

**Date**: 2026-06-27
**Status**: Proposed
**Author**: Toby Clemson

## Context

Accelerator's behaviour is delivered as skills running on the Claude Code runtime
(ADR-0051), with deterministic work delegated to a CLI (ADR-0045). A skill
represents one phase of work. Phases must hand work to one another, and the
durable artefacts they produce must have a home.

Two forces constrain how:

- The conversation context is bounded and costly. Large or long-lived artefacts
  cannot live only in the transcript — they are lost across context compaction
  and across separate skill invocations.
- Subagents do exploratory work in isolated context and return only summaries;
  their durable output needs somewhere to land.

This repo already runs on the filesystem-as-shared-memory model: phases
communicate **through the filesystem**, with a `meta/` directory as persistent
shared memory that skills read and write at predictable paths — `meta/work/`,
`meta/decisions/`, `meta/reviews/`, `meta/research/`, `meta/plans/` are populated.
This ADR records that model and **supersedes ADR-0027** (persist structured skill
outputs to `meta/`), carrying its decision forward: ADR-0027 established that
every skill producing structured output valuable to a later phase must write to
`meta/` (reversing the earlier "don't write to file" guidance in the review
skills) and that `meta/tmp/` stays purely ephemeral. That decision is the
persistence half of the same filesystem-as-message-bus principle, so the two are
unified here.

The luminosity original (lum ADR-0008) additionally split the filesystem into a
second top-level `content/` tree for shippable marketing deliverables (articles,
social, ads, imagery, video) and reasoned about coexisting with another plugin in
a shared `meta/`. Accelerator is a plugin, not a content product, and there is a
single `meta/` root here — so that `content/` half and the coexistence reasoning
are **out of scope and intentionally not ported**. Only the `meta/`-as-message-bus
half is kept, which is exactly the half that overlaps ADR-0027.

## Decision Drivers

- Bounded, costly conversation context — durable artefacts must outlive the
  transcript and survive compaction and separate invocations.
- Phases and subagents need a predictable, durable handoff channel that does not
  rely on conversational memory.
- A complete audit trail: every significant skill output should be recoverable
  later, for future sessions and teammates.
- A clean semantic boundary between persistent and ephemeral storage.
- Both humans and skills read these files; the layout must be legible and
  reviewable in version control.

## Considered Options

1. **Filesystem as message bus + knowledge corpus, single `meta/` root** — phases
   communicate by reading and writing predictable paths under `meta/`; structured
   outputs that a later phase, session, or teammate needs are persisted there
   (reviews under `meta/reviews/`), while `meta/tmp/` holds only ephemeral working
   data.
2. **Conversation as the channel** — pass artefacts inline through the transcript
   and rely on context to carry them between phases.
3. **Everything ephemeral in `meta/tmp/`** — both working data and durable
   outputs share one directory; persistence depends on someone remembering to
   copy files elsewhere.
4. **External store** — hold state in a database or service rather than the repo
   filesystem.

## Decision

We will use the **filesystem as the message bus and knowledge corpus**, under a
single `meta/` root. Phases communicate by reading and writing predictable paths,
not by passing artefacts through the conversation; subagents return summaries
while their durable output lands on disk.

`meta/` is persistent shared memory: the knowledge corpus and message bus. It
holds everything the process reads and writes, in categorised subdirectories —
durable decisions (ADRs under `meta/decisions/`) and transient working state
(work items, plans, research, reviews, notes). There is no separate deliverables
root, because the plugin ships no content product.

Carried forward from ADR-0027 (which this ADR supersedes): **every skill
producing structured output valuable to a later phase, future session, or another
team member must write to `meta/`.** Reviews are persisted to `meta/reviews/plans/`
and `meta/reviews/prs/` as numbered, never-replaced documents with appendable
re-review history. `meta/tmp/` is kept purely ephemeral and is always safe to
delete. This reverses the "don't write to file" guidance the review skills once
carried.

We chose option 1 because it gives phases a durable, cheap handoff that survives
compaction, keeps the conversation lean, and provides a complete, VCS-reviewable
audit trail. Option 2 was rejected: bounded context loses artefacts across
compaction and separate invocations. Option 3 was rejected: mixing durable
outputs with ephemeral scratch obscures what is safe to delete and what must be
kept. Option 4 was rejected: it breaks zero-setup, forfeits the VCS-reviewable
history the filesystem gives for free, and duplicates what the repo already
provides.

This ADR relates to **ADR-0001** (clear-context-phase foundation), which shares
the filesystem-communication concern but also bundles agent-separation and
token-budget decisions this ADR does not subsume — so ADR-0001 is **linked, not
superseded**, and remains in force.

## Consequences

### Positive

- Phases and subagents communicate durably and cheaply; handoffs survive context
  compaction and span separate skill invocations.
- A complete audit trail for all significant skill outputs; reviews and other
  structured outputs are recoverable later.
- All state is VCS-tracked — diffable, reviewable, and revertable.
- A clean semantic boundary: `meta/tmp/` is always safe to delete, the rest of
  `meta/` is committed knowledge.
- Unifies the message-bus and persist-structured-outputs decisions (ADR-0027)
  under one principle, removing the prior split across two ADRs.

### Negative

- Predictable paths are an implicit contract across skills; changing a path is a
  breaking change with no compiler to catch a missed reference.
- Filesystem state can drift from the conversation if a skill writes but fails to
  summarise what it wrote.
- Additional file I/O and disk usage on every skill run that persists output.

### Neutral

- The `meta/` root and its categorised subdirectories are fixed here; the exact
  subdirectory names are conventions, extensible as new artefact types appear.
- Skills address these paths via `${CLAUDE_PLUGIN_ROOT}` for plugin scripts and
  repo-relative paths for the corpus, consistent with existing conventions.
- Review artifacts follow an immutable-file-with-appendable-re-reviews pattern,
  distinct from the date-prefixed pattern used by research and plans (inherited
  from ADR-0027).

## References

- **Ported from luminosity** — original decision (lum ADR-0008, which also defined
  a `content/` deliverables tree dropped here):
  https://github.com/atomicinnovation/luminosity/blob/main/meta/decisions/ADR-0008-filesystem-as-message-bus-and-knowledge-corpus.md
- `meta/decisions/ADR-0027-persist-structured-skill-outputs-to-meta.md` —
  Superseded; its persist-structured-outputs decision is carried forward here.
- `meta/decisions/ADR-0051-skills-as-the-product.md` — Companion; deferred this
  mechanism to this decision.
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — Related;
  governs what work lives in skills vs. the CLI.
- `meta/decisions/ADR-0001-context-isolation-principles.md` — Related (not
  superseded); shares the filesystem-communication concern but bundles
  agent-separation and token-budget decisions this ADR does not subsume.
</content>
