---
title: Philosophy
description: >-
  Why Accelerator splits development into filesystem-mediated phases —
  the context-rot problem and the design that answers it.
---

Accelerator is built around one observation: **the quality of an LLM's
work degrades as its context window fills with material that is no
longer relevant.** Everything else in the plugin — the phase model, the
`meta/` directory, the subagent roster — follows from taking that
observation seriously.

## The problem: context rot

A long agentic coding session accumulates baggage. The transcript of an
exploratory grep, the full text of twenty files read while orienting,
three abandoned approaches, a digression into a flaky test — all of it
stays in the conversation, competing for the model's attention with the
thing that actually matters right now.

This degradation is gradual and easy to miss. The model does not fail
loudly; it gets slightly vaguer, drops a constraint it was told an hour
ago, or repeats work it already did. By the time the session reaches the
critical step — writing the code — the context is at its largest and its
noisiest, and the model is at its least sharp. Compaction helps a
conversation survive, but a summary of a cluttered context is still a
lossy view of a cluttered context.

The naive fix — start a fresh session — throws away the valuable part
(the findings) along with the noise (the process of finding them). The
interesting design question is how to keep one without the other.

## The response: phases that communicate through the filesystem

Accelerator splits development into discrete phases — research, plan,
implement — and forbids them from communicating through the
conversation. Each phase:

1. starts with a small, deliberate context;
2. does its work (which may involve reading a great deal);
3. distils the result into a structured Markdown document on disk;
4. ends.

The next phase reads that document — and only that document — as its
input. A research phase might read fifty files across the codebase, but
what reaches the planner is a structured findings document of a few
hundred lines: file paths, line references, verified behaviour,
constraints. The planner inherits the conclusions without inheriting
the fifty files. The implementer, in turn, inherits a phased plan with
explicit success criteria, not the planning debate that produced it.

This is why the split is by *phase* rather than by, say, module: each
phase has a natural distillation point where most of its working
context can be safely discarded, and a natural artefact (research
findings, a plan, a review) that carries everything the next phase
needs.

## `meta/` as persistent shared memory

The documents live in a `meta/` directory inside the repository —
research under `meta/research/`, plans under `meta/plans/`, work items
under `meta/work/`, decision records under `meta/decisions/`, and so
on. Every skill reads from and writes to predictable paths within it.

Treating the filesystem as the shared memory, rather than the
conversation, has consequences beyond context hygiene:

- **Work survives sessions.** A plan written on Tuesday can be
  implemented on Thursday in a brand-new session, because nothing the
  implementer needs lives in Tuesday's conversation.
- **Artefacts are inspectable and versionable.** They are plain
  Markdown with YAML frontmatter, so they can be read, diffed,
  reviewed, and committed like any other file — and browsed with the
  [visualiser](visualiser.md).
- **Documents link to each other.** Frontmatter records which work
  item a plan implements and which research it derives from, so the
  history of a change is a traversable graph, not an ephemeral chat
  log.
- **Progress is resumable.** Plans carry checkboxes;
  `implement-plan` picks up from the first unchecked item after any
  interruption.

The [Internals](internals.md) page documents the full anatomy of
`meta/`.

## Bounded subagent contexts

Phases keep the *sequence* of work lean; subagents keep each *step*
lean. When a skill needs exploratory work — "where does authentication
live?", "how do the existing tests structure fixtures?" — it does not
do the exploration in its own context. It spawns a subagent that runs
in an isolated context window with a restricted tool set, does the
messy searching and reading there, and returns only a focused summary.

The parent context therefore contains the question and the answer, but
never the exploration. Ten subagent investigations can feed a research
phase without the researcher's own context growing by more than ten
summaries — and because the subagents are independent, they run in
parallel.

## Locators and analysers

The subagent roster applies the same discipline one level down.
Agents are deliberately split into two kinds:

- **Locators** find *where* things are. They have search tools (Grep,
  Glob, LS) but **no Read tool**, so they physically cannot fill their
  context with file contents. They return organised lists of paths.
- **Analysers** understand *how* specific things work. They have Read,
  and they are pointed at the small set of files a locator (or the
  parent) already identified.

The separation prevents any single agent from needing to both search
broadly and read deeply — the combination that inflates a context
fastest. A broad question fans out cheaply through locators; deep
reading is spent only where it is known to matter. The same split is
applied across three domains: the codebase, the `meta/` document
corpus, and (for design work) a running application driven through a
browser.

## Human review where it is cheapest

The phase boundaries are also review points, and they are ordered by
leverage. A flawed plan costs minutes to fix at review time and hours
to fix mid-implementation; flawed research quietly corrupts everything
downstream. So the workflow concentrates human attention — and
dedicated review skills such as `review-plan` and `stress-test-plan` —
on the research and plan artefacts, *before* implementation begins.
Because those artefacts are self-contained documents rather than a
scrollback of conversation, reviewing them is genuinely practical.

## The trade-off, stated plainly

This design spends keystrokes to buy accuracy. Writing findings to
disk, reading them back, and re-establishing context at each phase
boundary has overhead a single long conversation does not. For a
trivial change, that overhead is not worth it — which is why
Accelerator is a toolkit of skills you can enter at any point, not a
mandatory pipeline. The bet is that for any change big enough to need
research and a plan, a sequence of small, sharp contexts beats one
large, degraded one — and that the artefact trail left behind (research,
plans, decisions) is valuable in itself.

To see the model in motion, follow the [Full Workflow](workflow.md) or
read the worked [case study](case-study.md) of a real change shipped
with Accelerator.
