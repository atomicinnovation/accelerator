---
type: codebase-research
id: "2026-06-27-ask-user-question-opportunities"
title: "Research: AskUserQuestion Upgrade Opportunities Across All Skills"
date: "2026-06-27T13:32:04+00:00"
author: "John Cowie Del Corral"
producer: research-codebase
status: complete
topic: "Skills that ask the user questions via plain text where AskUserQuestion tool could be used instead"
tags: [research, codebase, skills, ux, ask-user-question, review-plan, respond-to-pr, review-pr, refine-work-item]
revision: "1180b25e8710a3afe4a0080d02bccd24f439b95b"
repository: accelerator
last_updated: "2026-06-27T13:32:04+00:00"
last_updated_by: "John Cowie Del Corral"
schema_version: 1
---

# Research: AskUserQuestion Upgrade Opportunities Across All Skills

**Date**: 2026-06-27T13:32:04+00:00
**Author**: John Cowie Del Corral
**Git Commit**: 1180b25e8710a3afe4a0080d02bccd24f439b95b
**Branch**: feat/review-plan-ask-user-question-lens-confirmation
**Repository**: accelerator

## Research Question

The current branch (`feat/review-plan-ask-user-question-lens-confirmation`) replaced a plain-text
"Shall I proceed?" confirmation in `review-plan/SKILL.md` with the structured `AskUserQuestion` tool
(two options: Proceed / Specify something else). Can the same improvement be applied elsewhere across
the skill library?

## Summary

The `AskUserQuestion` tool is well-suited for any skill interaction that presents a bounded,
enumerable set of choices — whether binary (yes/no, proceed/adjust) or multi-option (e.g. menu
actions, conflict resolution strategies). Across the full skill library, **51 distinct interaction
points** were identified as candidates for this upgrade. They cluster into four patterns:

1. **Lens-selection confirmation** — the exact same pattern as the branch change; two skills remain unupgraded.
2. **Action menus** — post-review or post-analysis numbered option lists that map directly to
   AskUserQuestion options.
3. **Binary confirmations** — "y/N" or "Shall I proceed?" gates before irreversible actions.
4. **Bounded multi-select** — presenting a dynamically generated numbered list from which the user
   picks items.

The findings are ordered by priority: same-pattern first, then high-frequency skills, then the
consistent integration-skill pattern.

---

## Detailed Findings

### Priority 1: Same Pattern as the Branch Change (Lens Selection)

These two skills have **the exact same lens-selection-then-confirm flow** as `review-plan` and
should be updated in the same pass.

#### `skills/github/review-pr/SKILL.md` — Lines 237–258
- Current: presents lens selection list, ends with `"Shall I proceed, or would you like to adjust
  the selection?"`, then "Wait for confirmation before spawning reviewers."
- Same pattern as `review-plan`; replace with `AskUserQuestion`:
  - Option 1 (recommended): **Proceed** — run review with selected lenses
  - Option 2: **Specify something else** — adjust lens selection

#### `skills/work/review-work-item/SKILL.md` — Lines 147–155
- Current: "present the selection briefly… then wait for confirmation before spawning reviewers.
  Example: `I'll review this work item through all work item lenses (…). Shall I proceed?`"
- Same two-option structure as review-plan; identical fix applies.

---

### Priority 2: Action Menus (Post-Review / Post-Analysis)

Numbered option lists presented after analysis is complete — the user picks what happens next.
These are the highest-value multi-option upgrades.

#### `skills/github/review-pr/SKILL.md` — Lines 586–593
- Current: 5 numbered options after presenting the review:
  1. Post the review
  2. Change the verdict
  3. Edit or remove specific inline comments before posting
  4. Discuss findings in more detail
  5. Re-run specific lenses with adjusted focus
- Strong AskUserQuestion candidate; all 5 options are discrete and bounded.

#### `skills/github/review-pr/SKILL.md` — Lines 651–652
- Current: "Ask which verdict they prefer (APPROVE, COMMENT, REQUEST_CHANGES)"
- Exactly 3 fixed options — ideal AskUserQuestion.

#### `skills/decisions/review-adr/SKILL.md` — Lines 168–174
- Current: 3 options — Accept / Reject / Revise — ending with `"What would you like to do?"`
- 3 discrete, fixed options; strong candidate.

#### `skills/work/review-work-item/SKILL.md` — Lines 434–443
- Current: 4 numbered options after review preview:
  1. Proceed to address findings
  2. Change verdict
  3. Discuss specific findings
  4. Re-run specific lenses
- Matches the review-pr post-review pattern exactly.

#### `skills/work/extract-work-items/SKILL.md` — Lines 207–219
- Current: 4 options per candidate — enrich / accept as-is / skip / accept remaining as-is.
- 4 discrete options; strong candidate.

#### `skills/work/extract-work-items/SKILL.md` — Lines 266–277
- Current: 4 options — approve / revise \<instructions\> / skip / accept as-is.
- Options 1, 3, 4 are fully structured; option 2 is a hybrid (structured trigger + free-text
  instructions). Could be handled as "Revise" option with a follow-up free-text prompt.

#### `skills/github/respond-to-pr/SKILL.md` — Lines 328–333
- Current: 3 options for disagreement items — push back / implement / propose alternative — ending
  with `"What would you prefer?"`
- 3 discrete, fixed options.

---

### Priority 3: Binary Confirmations Before Irreversible Actions

All of these are simple yes/no gates. The AskUserQuestion pattern adds two labelled options
(e.g. **Proceed** / **Cancel**) rather than requiring the user to type "y" or "n".

#### `skills/vcs/commit/SKILL.md` — Lines 33–36
- Current: `"I plan to create [N] commit(s) with these changes. Shall I proceed?"`
- High-frequency, high-visibility skill — committing is the most common skill invocation.
- Options: **Proceed** (recommended) / **Cancel**

#### `skills/github/respond-to-pr/SKILL.md` — Lines 311–313
- Current: `"Shall I proceed with this change?"` (before applying a single agreed-upon fix)
- Options: **Proceed** (recommended) / **Skip this item**

#### `skills/github/respond-to-pr/SKILL.md` — Lines 344–346
- Current: `"Shall I post this response, or would you like to adjust it?"`
- Options: **Post as-is** (recommended) / **Adjust response**

#### `skills/github/respond-to-pr/SKILL.md` — Lines 455–456
- Current: `"Would you like me to push these changes to the remote?"`
- Options: **Push to remote** (recommended) / **Skip for now**

#### `skills/github/respond-to-pr/SKILL.md` — Lines 459–462
- Current: `"Would you like me to re-request review from them?"`
- Options: **Re-request review** (recommended) / **Skip**

#### `skills/github/respond-to-pr/SKILL.md` — Lines 60–63
- Current: warn PR is closed/merged and ask whether to proceed.
- Options: **Proceed anyway** / **Abort** (recommended)

#### `skills/github/respond-to-pr/SKILL.md` — Lines 72–74
- Current: warn branch mismatch and ask if user wants to switch.
- Options: **Switch to PR branch** (recommended) / **Stay on current branch**

#### `skills/github/review-pr/SKILL.md` — Lines 144–147
- Current: when diff is empty, inform user and ask whether to proceed with description-only review.
- Options: **Proceed with description review** / **Abort**

#### `skills/planning/stress-test-plan/SKILL.md` — Lines 139–165
- Current: `"Would you like me to update the plan with these changes?"`
- Options: **Update the plan** (recommended) / **Leave unchanged**

#### `skills/research/conduct-spike/SKILL.md` — Lines ~194
- Current: "Confirm the synthesis with the user before recording it."
- Options: **Record synthesis** (recommended) / **Revise before recording**

#### `skills/decisions/create-adr/SKILL.md` — Lines 132–139
- Current: `"Here's my draft ADR. Please review and let me know if you'd like any changes before I
  write it to disk."` + "Wait for user approval."
- Options: **Write to disk** (recommended) / **Request changes**

#### `skills/work/create-work-item/SKILL.md` — Lines 529–537
- Current: `Push to <tracker> now? [y/N]`
- Options: **Push to \<tracker\>** (recommended) / **Skip**

#### `skills/work/create-work-item/SKILL.md` — Lines 619–637
- Current: `Proceed? (y/n)` before overwriting in enrich-existing mode.
- Options: **Proceed** (recommended) / **Cancel**

#### `skills/work/refine-work-item/SKILL.md` — Lines 129–133
- Current: `bug/spike work items don't typically decompose — are you sure? (y/n)`
- Options: **Proceed anyway** / **Cancel** (recommended)

#### `skills/work/refine-work-item/SKILL.md` — Lines 167–172
- Current: `This will allocate N work item numbers and write N files… Proceed? (y/n)`
- Options: **Proceed** (recommended) / **Cancel**

#### `skills/work/refine-work-item/SKILL.md` — Lines 355–360
- Current: show diff for size change, require explicit y/n confirmation.
- Options: **Apply size change** / **Cancel**

#### `skills/work/refine-work-item/SKILL.md` — Lines 418–419
- Current: `"Would you like to run /review-work-item on this work item now?"`
- Options: **Run review** / **Skip**

#### `skills/work/review-work-item/SKILL.md` — Lines 471–476
- Current: `"The work item has been updated. Would you like me to run another review pass?"`
- Options: **Run another review** / **Done**

#### `skills/work/stress-test-work-item/SKILL.md` — Lines 144–157
- Current: `"Would you like me to update the work item with these changes?"`
- Options: **Update work item** (recommended) / **Leave unchanged**

#### `skills/work/update-work-item/SKILL.md` — Lines 142–145
- Current: `date records the work item's creation time and is typically not edited. Proceed anyway? (y/n)`
- Options: **Proceed anyway** / **Cancel** (recommended)

#### `skills/work/update-work-item/SKILL.md` — Lines 220–225
- Current: show diff then `"Apply these changes? (y/n)"`
- Options: **Apply changes** (recommended) / **Cancel**

#### `skills/work/sync-work-items/SKILL.md` — Lines 176–182
- Current: `N local files will be overwritten from remote. Proceed? [y/N]`
- Options: **Proceed** / **Abort** (recommended)

#### `skills/work/sync-work-items/SKILL.md` — Lines 289–298
- Current: `N untracked remote issues will be created. Proceed? [y/N]`
- Options: **Proceed** / **Abort** (recommended)

---

### Priority 4: Bounded Multi-Select from a Dynamic List

These present a numbered list of discovered items (documents, ADRs, etc.) and ask the user to select
from it. AskUserQuestion supports dynamic options lists, making these viable candidates — though
the implementation requires building the options array at runtime.

#### `skills/decisions/review-adr/SKILL.md` — Lines 42–69
- Current: shows a numbered list of discovered ADRs, ends with `"Which ADR would you like to
  review? (enter number or path)"`
- Dynamic list of ADRs; can be populated as AskUserQuestion options.

#### `skills/decisions/extract-adrs/SKILL.md` — Lines 43–53
- Current: `"You can: 1. Specify documents… 2. Let me scan all… Which would you prefer?"`
- Exactly 2 fixed options — one of the simplest possible upgrades.

#### `skills/decisions/extract-adrs/SKILL.md` — Lines 62–76
- Current: numbered list of discovered documents, `"Which documents should I scan for decisions?"`
- Dynamic list; multi-select via AskUserQuestion with `multiSelect: true`.

#### `skills/decisions/extract-adrs/SKILL.md` — Lines 99–115
- Current: numbered list of discovered decisions, `"Which decisions would you like to capture as ADRs?"`
- Dynamic list; multi-select.

#### `skills/decisions/extract-adrs/SKILL.md` — Lines 136–146
- Current: per-draft `"Does this look good? (yes / revise / skip / approve all remaining)"`
- 4 fixed options; strong candidate.

#### `skills/notes/create-note/SKILL.md` — Lines 68–72
- Current: `"Does <artifact> own this note as its parent, or is it just related? [owns / related]"`
- Exactly 2 fixed options.

#### `skills/github/describe-pr/SKILL.md` — Line 44
- Current: shows list of open PRs from `gh pr list`, asks user which to describe.
- Dynamic list of PRs; can be presented as AskUserQuestion options.

#### `skills/work/create-work-item/SKILL.md` — Lines 246–253
- Current: 3 numbered options when a similar work item exists — proceed new / update existing /
  create linked to existing.
- 3 fixed options.

#### `skills/work/refine-work-item/SKILL.md` — Lines 99–117
- Current: 5 numbered operations — decompose / enrich / sharpen / size / link. Multi-select allowed.
- 5 fixed options; `multiSelect: true` variant of AskUserQuestion.

#### `skills/work/refine-work-item/SKILL.md` — Lines 136–141
- Current: `append / skip / cancel` for existing children on decompose.
- 3 fixed options.

#### `skills/work/refine-work-item/SKILL.md` — Lines 309–317
- Current: `replace / append / skip` for Technical Notes on enrich.
- 3 fixed options.

#### `skills/work/refine-work-item/SKILL.md` — Lines 377–384
- Current: `replace / append / skip` for dependencies on link operation.
- 3 fixed options.

#### `skills/work/extract-work-items/SKILL.md` — Lines 108–124
- Current: numbered list of discovered documents, `"Which documents should I scan?"`
- Dynamic list; multi-select.

#### `skills/work/extract-work-items/SKILL.md` — Lines 144–160
- Current: numbered candidates list, `"Which items would you like to create work items for?"`
- Dynamic list; multi-select.

#### `skills/work/sync-work-items/SKILL.md` — Lines 206–213
- Current: `"Type 'remote' to OVERWRITE… 'local' to push… 'skip' to leave unchanged. [remote/local/skip]"`
- 3 fixed, clearly labelled options.

#### `skills/work/sync-work-items/SKILL.md` — Lines 246–248
- Current: `Push <id> "<title>" to <tracker>? [y/N]  (a = push all remaining, d = decline all remaining)`
- Binary yes/no with two shortcut tokens; the shortcuts (push all / decline all) could be surfaced
  as additional AskUserQuestion options.

---

### Priority 5: Integration Skills — Consistent Confirmation Pattern

All six integration skills (Jira: create, transition, update; Linear: create, transition, update)
share an identical confirmation gate:

```
Send this to <tracker>? Reply **y** to confirm, **n** to revise, anything else to abort.
```

All six are strong AskUserQuestion candidates. The recommended option should be **Confirm &
send**, with a second option **Revise**. The "anything else to abort" branch can become a third
option **Abort** or be folded into the dismiss/cancel behaviour.

Files:
- `skills/integrations/jira/create-jira-issue/SKILL.md:213–215` (also line 95: `[y/N]` variant)
- `skills/integrations/jira/transition-jira-issue/SKILL.md:130–131`
- `skills/integrations/jira/transition-jira-issue/SKILL.md:94–97` (ambiguous transition picker — multi-option)
- `skills/integrations/jira/update-jira-issue/SKILL.md:123–125`
- `skills/integrations/linear/create-linear-issue/SKILL.md:72–74`
- `skills/integrations/linear/transition-linear-issue/SKILL.md:53–55`
- `skills/integrations/linear/update-linear-issue/SKILL.md:56–58`

---

### Priority 6: `respond-to-pr` Preferences Questionnaire

#### `skills/github/respond-to-pr/SKILL.md` — Lines 256–278
- Current: a four-question preferences survey presented before work begins:
  1. Working mode: Guided / Express
  2. Commit strategy: per-item / per-category / at the end
  3. Thread resolution: auto-resolve after responding / leave threads open
  4. Item ordering: one ordered approach vs another
- Each sub-question maps cleanly to 2–3 discrete options. This is the most complex upgrade in the
  list — could be delivered as 4 sequential AskUserQuestion calls (one per preference dimension) or
  held as a single complex call (though AskUserQuestion supports multiple questions per invocation).

---

### Skills With NO Candidates (Only Free-Text Input)

The following skills were audited and have only open-ended free-text interactions — not suitable for
AskUserQuestion:

- `skills/planning/implement-plan/SKILL.md` — mismatches ask "How should I proceed?" (open)
- `skills/planning/validate-plan/SKILL.md` — no explicit user question gates
- `skills/research/research-codebase/SKILL.md` — "any follow-ups?" (weak)
- `skills/research/research-issue/SKILL.md` — "deeper investigation?" (weak)
- `skills/planning/create-plan/SKILL.md` — lines 282–291 (open feedback prompts)
- `skills/work/update-work-item/SKILL.md` — most interactions are free-text field input
- `skills/integrations/jira/*/SKILL.md` — "What would you like to change?" flows (open)
- `skills/decisions/create-adr/SKILL.md` — clarifying questions before drafting (open narrative)
- `skills/notes/create-note/SKILL.md` — initial "what would you like to note?" (open)

---

### `create-plan` Candidates Worth Noting

Two `create-plan` interactions were flagged as candidates but warrant discussion:

#### `skills/planning/create-plan/SKILL.md` — Lines 167–183 (design option selection)
- Current: presents Option A / Option B with pros/cons, asks "Which approach aligns best?"
- AskUserQuestion candidate: Yes, when the skill generates exactly 2–4 named options.
- Caveat: the options are dynamically generated content, not a fixed menu — implementation requires
  the model to construct the options array at runtime, which is doable.

#### `skills/planning/create-plan/SKILL.md` — Lines 191–203 (plan phasing approval)
- Current: presents phasing plan as numbered list, asks "Does this phasing make sense? Should I
  adjust the order or granularity?"
- AskUserQuestion candidate: Yes — binary proceed/adjust with an optional free-text follow-up on
  "adjust".

---

## Code References

| File | Lines | Pattern | Priority |
|---|---|---|---|
| `skills/github/review-pr/SKILL.md` | 237–258 | Lens-select confirm (same as branch) | 1 |
| `skills/work/review-work-item/SKILL.md` | 147–155 | Lens-select confirm (same as branch) | 1 |
| `skills/github/review-pr/SKILL.md` | 586–593 | 5-option post-review menu | 2 |
| `skills/github/review-pr/SKILL.md` | 651–652 | Verdict 3-option choice | 2 |
| `skills/decisions/review-adr/SKILL.md` | 168–174 | Accept/Reject/Revise 3-option | 2 |
| `skills/work/review-work-item/SKILL.md` | 434–443 | 4-option post-review menu | 2 |
| `skills/work/extract-work-items/SKILL.md` | 207–219 | 4-option per-candidate menu | 2 |
| `skills/github/respond-to-pr/SKILL.md` | 328–333 | 3-option disagreement handler | 2 |
| `skills/vcs/commit/SKILL.md` | 33–36 | "Shall I proceed?" commit confirm | 3 |
| `skills/github/respond-to-pr/SKILL.md` | 311–313 | "Shall I proceed?" per-fix | 3 |
| `skills/github/respond-to-pr/SKILL.md` | 344–346 | Post / Adjust response | 3 |
| `skills/github/respond-to-pr/SKILL.md` | 455–456 | Push to remote? | 3 |
| `skills/github/respond-to-pr/SKILL.md` | 459–462 | Re-request review? | 3 |
| `skills/github/respond-to-pr/SKILL.md` | 60–63 | Closed PR proceed? | 3 |
| `skills/github/respond-to-pr/SKILL.md` | 72–74 | Branch mismatch switch? | 3 |
| `skills/github/review-pr/SKILL.md` | 144–147 | Empty diff proceed? | 3 |
| `skills/planning/stress-test-plan/SKILL.md` | 139–165 | Update plan? | 3 |
| `skills/research/conduct-spike/SKILL.md` | ~194 | Confirm synthesis | 3 |
| `skills/decisions/create-adr/SKILL.md` | 132–139 | Approve ADR draft | 3 |
| `skills/work/create-work-item/SKILL.md` | 529–537 | Push to tracker? | 3 |
| `skills/work/create-work-item/SKILL.md` | 619–637 | Proceed overwrite? | 3 |
| `skills/work/refine-work-item/SKILL.md` | 129–133 | Bug/spike decompose confirm | 3 |
| `skills/work/refine-work-item/SKILL.md` | 167–172 | Large decompose proceed? | 3 |
| `skills/work/refine-work-item/SKILL.md` | 355–360 | Size change confirm | 3 |
| `skills/work/refine-work-item/SKILL.md` | 418–419 | Run review after ops? | 3 |
| `skills/work/review-work-item/SKILL.md` | 471–476 | Re-review after edits? | 3 |
| `skills/work/stress-test-work-item/SKILL.md` | 144–157 | Update work item? | 3 |
| `skills/work/update-work-item/SKILL.md` | 142–145 | Date field override confirm | 3 |
| `skills/work/update-work-item/SKILL.md` | 220–225 | Apply changes? | 3 |
| `skills/work/sync-work-items/SKILL.md` | 176–182 | Overwrite gate | 3 |
| `skills/work/sync-work-items/SKILL.md` | 289–298 | Create untracked issues? | 3 |
| `skills/decisions/extract-adrs/SKILL.md` | 43–53 | Specify docs vs scan all | 4 |
| `skills/decisions/extract-adrs/SKILL.md` | 62–76 | Which docs to scan? | 4 |
| `skills/decisions/extract-adrs/SKILL.md` | 99–115 | Which decisions to capture? | 4 |
| `skills/decisions/extract-adrs/SKILL.md` | 136–146 | Per-draft yes/revise/skip/all | 4 |
| `skills/notes/create-note/SKILL.md` | 68–72 | Owns vs related | 4 |
| `skills/github/describe-pr/SKILL.md` | 44 | Which PR to describe? | 4 |
| `skills/work/create-work-item/SKILL.md` | 246–253 | Similar item: 3-option | 4 |
| `skills/work/refine-work-item/SKILL.md` | 99–117 | Refinement menu (5-op, multi) | 4 |
| `skills/work/refine-work-item/SKILL.md` | 136–141 | Append/skip/cancel children | 4 |
| `skills/work/refine-work-item/SKILL.md` | 309–317 | Replace/append/skip tech notes | 4 |
| `skills/work/refine-work-item/SKILL.md` | 377–384 | Replace/append/skip deps | 4 |
| `skills/work/extract-work-items/SKILL.md` | 108–124 | Which documents to scan? | 4 |
| `skills/work/extract-work-items/SKILL.md` | 144–160 | Which candidates to extract? | 4 |
| `skills/work/sync-work-items/SKILL.md` | 206–213 | remote/local/skip conflict | 4 |
| `skills/work/sync-work-items/SKILL.md` | 246–248 | Push item (with a/d shortcuts) | 4 |
| `skills/decisions/review-adr/SKILL.md` | 42–69 | Which ADR to review? | 4 |
| `skills/integrations/jira/create-jira-issue/SKILL.md` | 95, 213–215 | Send to Jira? | 5 |
| `skills/integrations/jira/transition-jira-issue/SKILL.md` | 94–97, 130–131 | Which transition / send? | 5 |
| `skills/integrations/jira/update-jira-issue/SKILL.md` | 123–125 | Send to Jira? | 5 |
| `skills/integrations/linear/create-linear-issue/SKILL.md` | 72–74 | Create in Linear? | 5 |
| `skills/integrations/linear/transition-linear-issue/SKILL.md` | 53–55 | Transition? | 5 |
| `skills/integrations/linear/update-linear-issue/SKILL.md` | 56–58 | Apply to Linear? | 5 |
| `skills/github/respond-to-pr/SKILL.md` | 256–278 | 4-question preferences survey | 6 |
| `skills/planning/create-plan/SKILL.md` | 167–183, 191–203 | Design opts + phasing confirm | — |

---

## Architecture Insights

**The improvement is a systematic pattern, not a one-off.** Every skill that asks a bounded
question via plain text is a candidate. The pattern applies across all skill categories with no
exceptions — planning, review, work items, integrations, VCS, research, and decisions all have
examples.

**Three sub-patterns dominate:**

1. **`Reply **y** to confirm, **n** to revise`** — the integration skill idiom. All 6 integration
   skills share this exact phrasing. These are the fastest wins: identical change applied 6 times.

2. **Post-analysis action menus** — 5- or 4-option numbered lists after review/stress-test output.
   These are the highest UX impact: structured options are much easier to act on than "type a
   number".

3. **Pre-action safety gates** — `Proceed? (y/n)` before writes, pushes, or overwriting. Applying
   AskUserQuestion here also makes the *label* of the safe option explicit ("Abort" or "Cancel" as
   the recommended choice when the operation is destructive), which is clearer than `[y/N]`.

**`review-pr` is the most immediate follow-on.** It has 4 upgrade points (lens confirm, empty-diff
confirm, post-review menu, verdict picker) and the lens confirm is an exact copy of the change
already made in `review-plan`. It should be updated in the same PR or the next one.

**`respond-to-pr` has the most candidates (8)** but is also the most interactive skill — it may
benefit from a dedicated pass since several interactions are tightly coupled in the review loop.

## Open Questions

1. **Multi-question invocation**: AskUserQuestion supports multiple questions per call. For
   `respond-to-pr`'s 4-question preferences survey (lines 256–278), should these be bundled into
   one AskUserQuestion call or kept as sequential calls? One call reduces round-trips but may feel
   overwhelming.

2. **Multi-select lists**: `extract-work-items`, `extract-adrs`, `refine-work-item` all need
   multi-select from dynamic lists. Verify the AskUserQuestion tool supports `multiSelect: true`
   with dynamically constructed option arrays before implementation.

3. **Shortcut tokens in sync**: `sync-work-items` line 246 uses `a = push all` and `d = decline
   all` as fast-path shortcuts within a per-item loop. These could become additional AskUserQuestion
   options, but the loop-level semantics (apply to remaining items) need careful wording.

4. **Free-text revision branch**: Many "proceed/revise" patterns need a free-text follow-up when
   the user picks "Revise". The AskUserQuestion tool handles this by having Claude prompt for free
   text after the structured choice — confirm this is the right model before implementing.
