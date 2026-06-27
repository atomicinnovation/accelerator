---
type: plan
id: "2026-06-27-ask-user-question-skill-upgrades"
title: "AskUserQuestion Skill Upgrades (Priorities 1–3) Implementation Plan"
date: "2026-06-27T16:14:08+00:00"
author: "John Cowie Del Corral"
producer: create-plan
status: draft
derived_from: ["codebase-research:2026-06-27-ask-user-question-opportunities"]
tags: [skills, ux, ask-user-question, review-pr, review-work-item, respond-to-pr, commit]
revision: "e72cb30556fe3a3caaf952092d8056c932a2baa9"
repository: accelerator
last_updated: "2026-06-27T16:14:08+00:00"
last_updated_by: "John Cowie Del Corral"
schema_version: 1
---

# AskUserQuestion Skill Upgrades (Priorities 1–3) Implementation Plan

## Overview

Replace all plain-text user questions in the skill library (Priorities 1–3 from
the research document) with structured `AskUserQuestion` tool calls.

**Constraint discovered during implementation**: `AskUserQuestion` requires a
minimum of 2 options. The initial pattern (1 explicit option + built-in "Other")
was invalid. The corrected pattern for binary confirmations is always 2 explicit
options — a positive and a negative — with no reliance on the implicit "Other".

This plan covers 51 interaction points across 13 SKILL.md files, organised into
four independently mergeable phases grouped by pattern similarity.

## Current State Analysis

The `review-plan` skill establishes the reference pattern (corrected after the
`Invalid option parameter` error discovered in practice):

```
Then use the `AskUserQuestion` tool to ask the user whether to proceed, with
two options:

1. **Yes, use the proposed lenses** — run the review with the selected lenses
2. **No, specify which lenses to use** — adjust the selection before running
```

All skills in scope still use one of three plain-text idioms:
1. `"Shall I proceed?"` / `"Would you like to…?"` with no options
2. A numbered markdown list followed by `"What would you prefer?"`
3. `Reply **y** to confirm, **n** to revise, anything else to abort`

None of these present the structured UI that `AskUserQuestion` provides.

## Desired End State

Every bounded user question across Priorities 1–3 uses `AskUserQuestion`.

**Verification**: invoke each modified skill and confirm the tool's UI panel
appears at the expected interaction point. Specifically:
- Lens-selection skills: panel appears before reviewers are spawned.
- Action-menu skills: panel appears after analysis output is presented.
- Binary-confirmation skills: panel appears before the irreversible action.

## What We're NOT Doing

- Priority 4 (bounded multi-select from dynamic lists) — deferred.
- Priority 5 (integration skill `Reply **y**` pattern) — deferred.
- Priority 6 (`respond-to-pr` preferences questionnaire) — deferred.
- Any open-ended free-text questions (e.g. "What would you like to change?") —
  not candidates for AskUserQuestion.
- Automated tests — these are Markdown prompt files; correctness is verified
  by manual invocation.

## Implementation Approach

Each phase is a focused set of SKILL.md edits that can be shipped as an
independent PR. All changes follow the same substitution pattern:

**Binary confirmation** (2 options — minimum required by the tool):
```
Use the `AskUserQuestion` tool with two options:

1. **[Positive action]** — [brief description]
2. **[Negative / adjust action]** — [brief description]
```

**Action menu** (2–4 options):
```
Use the `AskUserQuestion` tool with the following options:

1. **[Option A]** — [description]
2. **[Option B]** — [description]
3. **[Option C]** — [description]

[If a 5th option existed: the 4-option limit means option N is folded into
"Other" — see per-file notes below.]
```

The `AskUserQuestion` tool has a hard limit of **4 options** per question.
Where the existing text lists 5 options, the two most closely related options
are merged or the least-common one is dropped to "Other".

---

## Phase 1: Lens-Selection Confirmations

**Files**: `skills/github/review-pr/SKILL.md`,
`skills/work/review-work-item/SKILL.md`

**Pattern**: Identical to the existing `review-plan` change — 1 explicit option
(**Proceed**) with "Other" for adjustments.

### Changes Required

#### 1. `skills/github/review-pr/SKILL.md` — Lines 255–258

**Current:**
```
Shall I proceed, or would you like to adjust the selection?
```

Wait for confirmation before spawning reviewers.

**Replace with:**
```
Then use the `AskUserQuestion` tool to ask the user whether to proceed, with
two options:

1. **Yes, use the proposed lenses** — run the review with the selected lenses
2. **No, specify which lenses to use** — adjust the selection before running

Wait for the user's answer before spawning reviewers. If they choose option 2,
ask which lenses they want, then re-present the updated selection using the
same `AskUserQuestion` pattern.
```

#### 2. `skills/work/review-work-item/SKILL.md` — Lines 143–155

**Current:**
```
Present the selection briefly — enumerate the chosen lenses with a one-line
focus each — then wait for confirmation before spawning reviewers.
```

Example ends with `"Shall I proceed?"`

**Replace the "wait for confirmation" instruction with:**
```
Then use the `AskUserQuestion` tool with two options:

1. **Yes, use the proposed lenses** — run the review with the selected lenses
2. **No, specify which lenses to use** — adjust the selection before running

Wait for the user's answer before spawning reviewers. If they choose option 2,
ask which lenses they want, then re-present the updated selection using the
same `AskUserQuestion` pattern.
```

### Success Criteria

#### Manual Verification:
- [ ] Run `/review-pr` on any open PR — `AskUserQuestion` panel appears before
  reviewers are spawned; the panel shows **Proceed** as the sole option plus the
  "Other" input.
- [ ] Entering text in "Other" triggers the adjustment loop.
- [ ] Run `/review-work-item` on any work item — same panel behaviour.

---

## Phase 2: Post-Analysis Action Menus

**Files**: `skills/github/review-pr/SKILL.md`,
`skills/work/review-work-item/SKILL.md`,
`skills/decisions/review-adr/SKILL.md`,
`skills/work/extract-work-items/SKILL.md`,
`skills/github/respond-to-pr/SKILL.md`

**Pattern**: Replace numbered markdown option lists with `AskUserQuestion`.
Where 5 options exist, the two most closely related are merged to respect the
4-option limit (see `review-pr` note below).

### Changes Required

#### 1. `skills/github/review-pr/SKILL.md` — Lines 586–593 (5-option post-review menu)

**Current:**
```
The review is ready. Would you like to:
1. Post the review? …
2. Change the verdict? …
3. Edit or remove specific inline comments before posting?
4. Discuss any findings in more detail?
5. Re-run specific lenses with adjusted focus?
```

**4-option limit resolution**: Options 4 (Discuss) and 5 (Re-run lenses) are
distinct enough to keep both. Drop "Discuss" to "Other" (users can type "discuss
X") and keep Post, Change verdict, Edit comments, Re-run lenses as 4 options.

**Replace with instruction to use `AskUserQuestion` with:**
1. **Post the review** — submit summary + inline comments with [verdict]
2. **Change the verdict** — currently: [verdict]
3. **Edit or remove inline comments** — modify before posting
4. **Re-run specific lenses** — adjust focus and re-review

"Other" handles discussion requests.

#### 2. `skills/github/review-pr/SKILL.md` — Lines 652–653 (verdict picker)

**Current:**
```
Ask which verdict they prefer (APPROVE, COMMENT, REQUEST_CHANGES)
```

**Replace with instruction to use `AskUserQuestion` with:**
1. **APPROVE** — approve the PR
2. **COMMENT** — leave a non-blocking comment review
3. **REQUEST_CHANGES** — request changes before merge

#### 3. `skills/decisions/review-adr/SKILL.md` — Lines 168–174

**Current:**
```
1. Accept
2. Reject
3. Revise

What would you like to do?
```
Wait for user decision.

**Replace with instruction to use `AskUserQuestion` with:**
1. **Accept** — mark the ADR as accepted
2. **Reject** — mark it as rejected (reason prompted after)
3. **Revise** — update the ADR before deciding

#### 4. `skills/work/review-work-item/SKILL.md` — Lines 434–443 (post-review menu)

**Current:**
```
Would you like to:
1. Proceed to address findings? …
2. Change the verdict? …
3. Discuss any specific findings in more detail?
4. Re-run specific lenses with adjusted focus?
```

**Replace with instruction to use `AskUserQuestion` with:**
1. **Address findings** — edit the work item to resolve issues
2. **Change the verdict** — currently: [verdict]
3. **Re-run specific lenses** — adjust focus and re-review

"Other" handles discussion requests (3 explicit options, keeping under the cap).

#### 5. `skills/work/extract-work-items/SKILL.md` — Lines 207–219 (per-candidate menu)

**Current:**
```
1. enrich
2. accept as-is
3. skip
4. accept remaining as-is
```

**Replace with instruction to use `AskUserQuestion` with:**
1. **Enrich** — improve this candidate before creating
2. **Accept as-is** — create work item from this candidate unchanged
3. **Skip** — skip this candidate
4. **Accept all remaining** — accept every remaining candidate as-is

#### 6. `skills/work/extract-work-items/SKILL.md` — Lines 266–277 (post-enrichment menu)

**Current:**
```
1. approve
2. revise <instructions>
3. skip
4. accept as-is
```

**4-option limit**: All 4 fit. Option 2 (Revise) is a hybrid — selecting it
prompts a follow-up free-text input for instructions.

**Replace with instruction to use `AskUserQuestion` with:**
1. **Approve** — use this enriched draft
2. **Revise** — re-enrich with additional instructions (prompted after)
3. **Skip** — skip this candidate entirely
4. **Accept as-is** — use the original un-enriched candidate

After the user selects **Revise**, prompt for the revision instructions as a
follow-up plain-text request (not a second AskUserQuestion).

#### 7. `skills/github/respond-to-pr/SKILL.md` — Lines 328–333 (disagreement handler)

**Current:**
```
Options:
1. Push back with this reasoning
2. Implement the suggestion anyway
3. Propose an alternative approach

What would you prefer?
```

**Replace with instruction to use `AskUserQuestion` with:**
1. **Push back** — post the draft reasoning as a response
2. **Implement anyway** — make the suggested change
3. **Propose alternative** — draft an alternative approach

### Success Criteria

#### Manual Verification:
- [ ] Run `/review-pr` on any PR, complete the review — `AskUserQuestion`
  panel appears with 4 post-review options (not a markdown list).
- [ ] Choosing "Change the verdict" triggers the 3-option verdict picker.
- [ ] Run `/review-adr` on any ADR — Accept/Reject/Revise panel appears.
- [ ] Run `/review-work-item` — post-review panel appears with 3 options.
- [ ] Run `/extract-work-items` — per-candidate 4-option panel appears; "Accept
  all remaining" closes the loop correctly.
- [ ] Post-enrichment panel shows Approve/Revise/Skip/Accept as-is; choosing
  Revise prompts for follow-up instructions.
- [ ] Run `/respond-to-pr` on a PR with disagreement items — 3-option panel
  appears for each disagreement.

---

## Phase 3: Binary Confirmations — VCS, GitHub & Planning Skills

**Files**: `skills/vcs/commit/SKILL.md`, `skills/github/respond-to-pr/SKILL.md`
(5 confirmations), `skills/github/review-pr/SKILL.md` (1), 
`skills/planning/stress-test-plan/SKILL.md`,
`skills/research/conduct-spike/SKILL.md`,
`skills/decisions/create-adr/SKILL.md`

**Pattern**: Each plain-text confirmation is replaced with a 1-option
`AskUserQuestion` (the action label), with "Other" as the implicit cancel/adjust
path. No "recommended" label on any option.

### Changes Required

#### 1. `skills/vcs/commit/SKILL.md` — Line 36

**Current:** `Ask: "I plan to create [N] commit(s) with these changes. Shall I proceed?"`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, proceed** — create [N] commit(s) with the staged changes
2. **No, cancel** — abort without committing

#### 2. `skills/github/respond-to-pr/SKILL.md` — Lines 59–63 (closed/merged PR)

**Current:** `inform the user and ask whether they still want to proceed`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, proceed anyway** — continue responding to a closed/merged PR
2. **No, abort** — exit the workflow

#### 3. `skills/github/respond-to-pr/SKILL.md` — Lines 72–74 (branch mismatch)

**Current:** `inform the user and ask if they want to switch`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, switch to PR branch** — check out [headRefName] before continuing
2. **No, stay on current branch** — continue without switching

#### 4. `skills/github/respond-to-pr/SKILL.md` — Lines 311–313 (per-fix confirm)

**Current:** `Shall I proceed with this change?`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, apply this change** — make the edit now
2. **No, skip this item** — leave it unaddressed and move on

#### 5. `skills/github/respond-to-pr/SKILL.md` — Lines 344–346 (post/adjust response)

**Current:** `Shall I post this response, or would you like to adjust it?`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Post as-is** — submit the draft response to GitHub
2. **Adjust first** — revise the response before posting

#### 6. `skills/github/respond-to-pr/SKILL.md` — Lines 455–456 (push to remote)

**Current:** `Would you like me to push these changes to the remote?`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, push now** — push committed changes to the remote
2. **No, skip** — leave changes unpushed

#### 7. `skills/github/respond-to-pr/SKILL.md` — Lines 459–465 (re-request review)

**Current:** `Would you like me to re-request review from them?`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, re-request** — re-request review from [reviewer list]
2. **No, skip** — leave review request as-is

#### 8. `skills/github/review-pr/SKILL.md` — Lines 145–147 (empty diff)

**Current:** `inform the user and ask whether to proceed with a review of the PR description and commits only`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, review description and commits only** — proceed without a diff
2. **No, abort** — exit without reviewing

#### 9. `skills/planning/stress-test-plan/SKILL.md` — Lines 164–165

**Current:** `Would you like me to update the plan with these changes?`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, update the plan** — apply the stress-test findings to the plan file
2. **No, leave unchanged** — keep the plan as-is

#### 10. `skills/research/conduct-spike/SKILL.md` — Around line 194

**Current:** `"Confirm the synthesis with the user before recording it."`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, record this synthesis** — write the outcome to the spike document
2. **No, revise first** — adjust the synthesis before recording

#### 11. `skills/decisions/create-adr/SKILL.md` — Lines 132–139

**Current:** `"Here's my draft ADR. Please review and let me know if you'd like any changes before I write it to disk."` + wait for approval.

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, write to disk** — save the ADR as shown
2. **No, revise first** — make changes before saving

### Success Criteria

#### Manual Verification:
- [ ] Run `/commit` — `AskUserQuestion` panel appears showing the commit plan
  with **Proceed** as the sole option.
- [ ] Run `/respond-to-pr` on a closed PR — panel appears with **Proceed anyway**;
  entering nothing or "cancel" in Other exits cleanly.
- [ ] Run `/respond-to-pr` on a PR from a different branch — branch-switch panel
  appears.
- [ ] Step through `/respond-to-pr` feedback items — per-fix **Proceed** and
  **Post this response** panels fire at the right moments.
- [ ] Reach end of `/respond-to-pr` — push panel appears, then re-request panel.
- [ ] Run `/review-pr` on a PR with an empty diff (draft PR) — description-only
  proceed panel appears.
- [ ] Run `/stress-test-plan` to completion — update plan panel appears.
- [ ] Run `/conduct-spike` to synthesis — record synthesis panel appears.
- [ ] Run `/create-adr` through to draft — write-to-disk panel appears.

---

## Phase 4: Binary Confirmations — Work Item Skills

**Files**: `skills/work/create-work-item/SKILL.md` (2),
`skills/work/refine-work-item/SKILL.md` (4),
`skills/work/review-work-item/SKILL.md` (1),
`skills/work/stress-test-work-item/SKILL.md` (1),
`skills/work/update-work-item/SKILL.md` (2),
`skills/work/sync-work-items/SKILL.md` (2)

**Pattern**: Same 1-option `AskUserQuestion` binary confirmation as Phase 3.

### Changes Required

#### 1. `skills/work/create-work-item/SKILL.md` — Lines 529–537 (push to tracker)

**Current:** `Push to <tracker> now? [y/N]`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, push to [tracker]** — create the issue in the external tracker now
2. **No, skip** — create locally only

#### 2. `skills/work/create-work-item/SKILL.md` — Lines 619–637 (enrich-existing overwrite)

**Current:** `Proceed? (y/n)`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, proceed** — overwrite the existing fields as shown
2. **No, cancel** — leave the work item unchanged

#### 3. `skills/work/refine-work-item/SKILL.md` — Lines 129–133 (bug/spike decompose)

**Current:** `bug/spike work items don't typically decompose — are you sure? (y/n)`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, proceed anyway** — decompose despite the work item type warning
2. **No, cancel** — abort the decompose operation

#### 4. `skills/work/refine-work-item/SKILL.md` — Lines 167–172 (large decompose gate)

**Current:** `This will allocate N work item numbers and write N files; … Proceed? (y/n)`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, proceed** — allocate [N] work item numbers and write [N] files
2. **No, cancel** — abort without writing any files

#### 5. `skills/work/refine-work-item/SKILL.md` — Lines 355–360 (size change confirm)

**Current:** show diff, require explicit y/n confirmation before Edit.

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, apply size change** — update the Size field as shown
2. **No, cancel** — leave Size unchanged

#### 6. `skills/work/refine-work-item/SKILL.md` — Lines 418–419 (offer review)

**Current:** `"Would you like to run /review-work-item on this work item now?"`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, run review** — invoke review-work-item on this work item
2. **No, done** — finish without reviewing

#### 7. `skills/work/review-work-item/SKILL.md` — Lines 471–476 (re-review after edits)

**Current:** `"The work item has been updated. Would you like me to run another review pass?"`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, run another review pass** — re-run all lenses on the updated work item
2. **No, done** — finish here

#### 8. `skills/work/stress-test-work-item/SKILL.md` — Lines 144–157 (update work item)

**Current:** `"Would you like me to update the work item with these changes?"`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, update the work item** — apply the stress-test findings
2. **No, leave unchanged** — keep the work item as-is

#### 9. `skills/work/update-work-item/SKILL.md` — Lines 142–145 (date field warning)

**Current:** `date records the work item's creation time and is typically not edited. Proceed anyway? (y/n)`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, proceed anyway** — edit the date field despite the warning
2. **No, cancel** — leave the date field unchanged

#### 10. `skills/work/update-work-item/SKILL.md` — Lines 220–225 (apply changes confirm)

**Current:** show diff then `"Apply these changes? (y/n)"`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, apply changes** — write the diff as shown to the work item file
2. **No, cancel** — discard the changes

#### 11. `skills/work/sync-work-items/SKILL.md` — Lines 176–182 (overwrite gate)

**Current:** `N local files will be overwritten from remote. Proceed? [y/N]`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, proceed** — overwrite [N] local files with remote versions
2. **No, abort** — cancel the sync

#### 12. `skills/work/sync-work-items/SKILL.md` — Lines 289–298 (untracked pull gate)

**Current:** `N untracked remote issues will be created. Proceed? [y/N]`

**Replace with instruction to use `AskUserQuestion` with:**
1. **Yes, proceed** — create [N] new local work items from remote
2. **No, abort** — cancel the sync

### Success Criteria

#### Manual Verification:
- [ ] Run `/create-work-item` and reach the push-to-tracker offer — panel appears.
- [ ] Run `/create-work-item` in enrich-existing mode — overwrite confirm panel
  appears before any fields are changed.
- [ ] Run `/refine-work-item --decompose` on a bug or spike — warning panel
  appears before decomposition begins.
- [ ] Run `/refine-work-item --decompose` on a large story — file-count warning
  panel appears.
- [ ] Run `/refine-work-item --size` with a size change — diff + confirm panel
  appears before the edit.
- [ ] Complete a `/refine-work-item` operation — "Run review" offer panel appears.
- [ ] Complete a `/review-work-item` edit loop — "Run another review" panel
  appears.
- [ ] Complete `/stress-test-work-item` — update offer panel appears.
- [ ] Run `/update-work-item` on the `date` field — warning + proceed panel appears.
- [ ] Run `/update-work-item` on any field — diff + apply confirm panel appears.
- [ ] Run `/sync-work-items --pull` with overwriteable files — overwrite gate
  panel appears.
- [ ] Run `/sync-work-items --pull` with untracked remote items — create gate
  panel appears.

---

## References

- Research document: `meta/research/codebase/2026-06-27-ask-user-question-opportunities.md`
- Reference implementation: `skills/planning/review-plan/SKILL.md` (commit `e72cb305`)
- AskUserQuestion tool constraints: max 4 options, min 2; `multiSelect` available
  but not used in Priorities 1–3.
