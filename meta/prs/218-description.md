---
type: "pr-description"
id: "218"
title: "Have explore-pr's tour stops say why, not what the code does"
date: "2026-11-14T18:35:39+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/218"
pr_number: 218
tags: ["explore-pr", "skills"]
revision: "4a83dd941e88ba8f9bb9f3a6263bb1211195324a"
repository: "accelerator"
last_updated: "2026-11-14T18:35:39+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Have explore-pr's tour stops say why, not what the code does

## Summary

Each stop in an explore-pr guided tour shows its code beside its text, and
the text mostly repeated what the code already said. The contract asked for
exactly that: a stop's `note` was defined as "what it does", and its `fit`
as who calls it. This PR replaces those fields with ones the code cannot
answer, links each stop to the design it is evidence for, and has the build
refuse a stop that narrates.

Stacked on #217.

## Changes

### What a stop says

- `note` and `check` are gone. A stop now carries `why` (the constraint
  that forced this shape, the alternative rejected here, what the lines
  cannot show), an optional `risk` (what breaks if it is wrong, how far it
  spreads, loudly or silently) and `checks`, each with a `question`, an
  optional `where` and the `problem` answer that would count against the
  change.
- `fit` stays but is re-scoped to the role the code plays in the design,
  not its call wiring, which the chapter's figures already draw.
- `carries` lists the ids of the decisions, abstractions, invariants and
  risks the stop is evidence for. A chapter risk gains an optional `id` so
  a stop can carry it.
- `focus` marks the one or two lines that matter, each with a few words,
  in place of a narrated walk through the block.

### Build (`scripts/build.py`)

- Warns on a stop with no `why`; one that carries nothing or carries an id
  nothing declares; a chapter decision or a high risk that no stop
  carries; a high risk with no `id`; a stop that states or carries a risk
  but has no `checks`; a `focus` line outside the stop's range; and a
  leftover `note` or `check`.
- Leaves a `note:`, not a warning, on a `why` that names four or more
  identifiers from its own lines.
- Leaves out a check with no `question` or no `problem`, and a focus with
  no `line`, and reports each.
- `--reanchor` moves a stop's `focus` lines with it.

### Page (`assets/page.html`, `assets/stop-evidence.js`)

- A stop shows what it carries as links, then its role, why, risk and
  checks. Each link opens the decision, abstraction, invariant or risk it
  names.
- Focus lines are marked in the stop's code with the remark beneath the
  line, kept within the visible width of the code panel.
- Every decision and abstraction card lists the stops that show it.
- A new page model, `stop-evidence.js`, resolves what each stop carries
  and which stops show each item. It is inlined into both layouts.

### Guidance

- `reference/content-model.md` gains a "What a stop says" section with the
  deletion test (could the reviewer have written this sentence from the
  lines shown?) and where a why may come from. A why no source states is
  asked as a question, not inferred.
- `reference/reading-pass-brief.md`, `SKILL.md` and `reference/page.md`
  follow.
- `examples/chapter-version-claims.json` has its three stops rewritten to
  the new shape, with focus lines checked against the source at that PR's
  revision.
- `evals/evals.json` gains a case for "the stops just repeat the code".

### Smoke check (`scripts/smoke.js`)

- A new probe checks that a stop names everything it carries, that a name
  opens what it names, and that every focus line is marked in its code.

## Context

There is no work item. The change follows from using the explore-pr page
on real pull requests and finding the stop text little help in analysing
the change.

## Testing

- [x] The skill's suite passes: `python3 -m pytest tests` in
  `.claude/skills/explore-pr`, 272 tests, including the node suite for
  `stop-evidence.js`.
- [x] `ruff format` and `ruff check` are clean on the skill.
- [x] A page built from the rewritten example chapter passes the smoke
  check in headless Chrome, and its tour screenshot was read.
- [x] The PR 188 review page was rebuilt with all 34 stops rewritten to
  the new shape: 0 warnings, 0 notes, and the smoke check passes without
  `--shots`.
- [ ] The bare `mise run` was not run.

## Notes for Reviewers

- **Existing workspaces fail the build** until their stops are rewritten:
  a leftover `note` or `check` is a warning, and both are no longer shown.
  That was chosen over rendering both shapes so that old narration cannot
  linger unnoticed.
- **A high risk must have an `id` and a stop that carries it.** This could
  be relaxed to a `note:` if it proves too strict.
- **The identifier threshold of four is a guess** drawn from one example
  chapter.
- **`smoke.js --shots` can stall.** Capturing the full-height screenshot
  of the longest PR 188 chapter hung headless Chrome twice. The same check
  passes without `--shots`. This PR does not fix it, and it may predate
  this change; stops are longer now, which makes it likelier.

https://claude.ai/code/session_01HEhrwWBovAtNmTKoz7ZARV
