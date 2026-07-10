---
title: Full Workflow
---

Accelerator is a toolkit, not a fixed pipeline. The skills are designed to fit
together end to end — from capturing work through to shipping a PR — but you
pick the parts you need. A bug fix might be `research-issue → create-plan →
implement-plan`; a larger feature might run the whole map below.

The spine is the [Development Loop](development-loop.md) — **research → plan →
implement**. The other families attach to it: some feed work in before research,
others capture decisions along the way and land the change after.

## The spine, end to end

Taken end to end, a full pass runs research → plan → implement → review
→ commit → PR. Each solid arrow is a hand-off through a document in
[`meta/`](philosophy.md), not through the conversation:

```mermaid
flowchart LR
  research["research-codebase"]
  plan["create-plan"]
  planreview["review-plan /<br/>stress-test-plan"]
  implement["implement-plan"]
  review["code review"]
  commit["commit"]
  pr["describe-pr →<br/>review-pr"]

  research -->|"research doc"| plan
  plan -->|"plan doc"| planreview
  planreview -->|"revised plan"| implement
  implement -->|"working tree"| review
  review --> commit
  commit --> pr
  planreview -.->|"findings"| plan
```

The review boxes are deliberate pauses: plan review happens before any
code exists (the cheapest point to change course), and PR review closes
the loop after the change lands on a branch.

## The full map

Each box below is a skill family from [All skills](reference/skills/index.md).

```mermaid
flowchart LR
  subgraph intake["Work Items · Issue Trackers · Design Convergence"]
    direction TB
    trackers["Issue trackers<br/>Jira · Linear"] <--> work["Work items"]
    design["Design convergence"] -->|extract-work-items| work
  end

  subgraph investigation["Investigation & Notes"]
    direction TB
    issue["research-issue"]
    spike["conduct-spike"]
    note["create-note"]
  end

  subgraph loop["Development Loop"]
    research["research-codebase"] --> create["create-plan"]
    create --> review["review-plan ⇄ stress-test-plan"]
    review --> implement["implement-plan"] --> validate["validate-plan"]
  end

  subgraph ship["VCS & PR"]
    commit["commit"] --> describe["describe-pr"]
    describe --> reviewpr["review-pr ⇄ respond-to-pr"]
  end

  adr["Architecture Decision<br/>Records"]

  work --> research
  issue --> research
  spike --> research
  note -.-> loop
  implement --> commit
  adr -.-> loop
```

## How the families fit together

Each heading below is a skill family from [All skills](reference/skills/index.md). They
are ordered by where they sit relative to the spine — what feeds work in, the
loop itself, and what lands the result.

**Work Items, Issue Trackers & Design Convergence — what gets worked on.**
[Work items](skills/work-items.md) are the unit of work. They can be created
locally, or synced both ways with a remote
[issue tracker](skills/issue-trackers.md) (Jira or Linear).
[Design convergence](skills/design-convergence.md) feeds the funnel from the
other side: it inventories a frontend, analyses gaps, and emits a document that
[`extract-work-items`](reference/skills/work/extract-work-items.md) turns into work
items.

**Investigation & Notes — understand before planning.**
Two skills feed the loop ahead of research when needed:
[`research-issue`](reference/skills/research/research-issue.md) for hypothesis-driven
bug investigation, and
[`conduct-spike`](reference/skills/research/conduct-spike.md) for time-boxed
uncertainty reduction. [`create-note`](reference/skills/notes/create-note.md)
captures observations at any point.

**Development Loop — the spine.**
The loop opens with
[`research-codebase`](reference/skills/research/research-codebase.md), which
investigates the code and writes a structured findings document.
[`create-plan`](reference/skills/planning/create-plan.md) builds a phased plan from
that research; [`review-plan`](reference/skills/planning/review-plan.md) and
[`stress-test-plan`](reference/skills/planning/stress-test-plan.md) iterate on its
quality before any code is written;
[`implement-plan`](reference/skills/planning/implement-plan.md) executes it phase by
phase; [`validate-plan`](reference/skills/planning/validate-plan.md) confirms the
result matches the plan. See the [Development Loop](development-loop.md) for
detail.

**Architecture Decision Records — captured as you go.**
[ADRs](skills/adrs.md) record architectural decisions made during planning or
implementation; [`extract-adrs`](reference/skills/decisions/extract-adrs.md) pulls them out of
existing research and plan documents.

**VCS & PR — land the change.**
The [VCS & PR workflow](skills/vcs-and-pr.md) closes the loop:
[`commit`](reference/skills/vcs/commit.md) through implementation, then
[`describe-pr`](reference/skills/github/describe-pr.md),
[`review-pr`](reference/skills/github/review-pr.md), and
[`respond-to-pr`](reference/skills/github/respond-to-pr.md) to get the change reviewed
and merged.

Every skill reads and writes the shared [`meta/`](philosophy.md) directory, so
phases hand off through the filesystem rather than the conversation. For the
complete catalogue, see [All skills](reference/skills/index.md).
