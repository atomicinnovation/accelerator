---
date: "2026-04-08T00:15:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-07-fix-tmp-directory-usage-in-pr-skills.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, correctness, code-quality, standards, usability, safety]
review_pass: 2
status: complete
---

## Plan Review: Fix /tmp Directory Usage in PR Skills

**Verdict:** COMMENT

The plan is well-scoped and architecturally sound, correctly identifying the
two-hop placeholder resolution chain as the key failure mode and proposing
minimal, convention-consistent fixes across two independent phases. The changes
follow established precedent from the `init` skill and are independently
verifiable. Two areas deserve attention before implementation: `describe-pr`
lacks a directory creation step that could cause failures for users who haven't
run `/init`, and the plan applies defensive substitution instructions to
`review-pr` but not to `describe-pr`, creating an asymmetry in the fix.

### Cross-Cutting Themes

- **Missing `mkdir -p` in describe-pr** (flagged by: Correctness, Safety) —
  The plan replaces `/tmp` (always exists) with `{tmp directory}` (may not
  exist) but adds no directory creation step to `describe-pr`. The `review-pr`
  skill already handles this with `mkdir -p` in Step 1.2, but `describe-pr`
  has no equivalent. This is the most impactful gap in the plan.

- **Asymmetric hardening between the two skills** (flagged by: Correctness,
  Code Quality, Standards, Usability) — Phase 2 adds explicit substitution
  instructions to `review-pr` but Phase 1 does not add equivalent instructions
  to `describe-pr`. The plan's own analysis identifies implicit placeholder
  resolution as fragile, yet only one of the two modified skills receives
  the guardrail.

- **Instructional guardrails are probabilistic** (flagged by: Architecture,
  Usability, Safety) — The fix relies on LLM instruction-following rather
  than structural enforcement. The plan explicitly acknowledges this tradeoff
  and the decision not to pursue preprocessor-level fixes, which is pragmatic
  for the current scope. Multiple lenses note this is acceptable but worth
  documenting as a known limitation.

### Tradeoff Analysis

- **Structural enforcement vs pragmatic scope**: Architecture and Usability
  both note that a programmatic template resolution mechanism would be more
  reliable, but the plan correctly bounds scope to the instructional approach.
  The tradeoff is well-reasoned — the recommendation is to document it
  explicitly as a known limitation rather than expand scope.

### Findings

#### Major

- 🟡 **Correctness**: Missing `mkdir -p` for tmp directory in describe-pr
  **Location**: Phase 1: Changes Required, Section 2-3
  The plan replaces `/tmp/pr-body-{number}.md` with
  `{tmp directory}/pr-body-{number}.md`, but `/tmp` is guaranteed to exist on
  all Unix systems while `meta/tmp` is not. The `review-pr` skill handles this
  with `mkdir -p` in Step 1.2, but no equivalent is added to `describe-pr`.
  Users who haven't run `/init` will get file-not-found errors.

- 🟡 **Usability**: LLM instruction-based substitution has no failure signal
  **Location**: Phase 2: Changes Required, Section 1 and 2
  If the LLM fails to follow the substitution instructions, artefacts silently
  land in `/tmp` or literal `{tmp directory}` appears in paths. There is no
  verification step or diagnostic output to help users identify the root cause.

#### Minor

- 🔵 **Correctness + Code Quality + Standards + Usability**: No explicit
  substitution instruction added to describe-pr
  **Location**: Phase 1: Changes Required
  Phase 2 adds hardening to `review-pr` but Phase 1 does not add an equivalent
  to `describe-pr`. The `init` skill's line 125 precedent shows substitution
  reminders are used even in single-agent contexts.

- 🔵 **Standards**: Instruction formatting departs from existing IMPORTANT
  pattern
  **Location**: Phase 2, Change 1
  The proposed `**IMPORTANT — path substitution**:` format uses an em-dash
  sub-label style not found elsewhere in the codebase. Existing skills use
  `**IMPORTANT**:` followed by instruction text.

- 🔵 **Code Quality**: Duplicated instructional text in review-pr
  **Location**: Phase 2: Changes Required, Section 1 and 2
  Two separate instruction blocks convey the same substitution message. If
  the set of path placeholders changes, both must be updated in lockstep.

- 🔵 **Architecture**: Natural-language guardrails address a systemic issue
  at a single site
  **Location**: Phase 2: Harden review-pr Template Variable Resolution
  The bold-label-to-placeholder convention is used across all skills. If
  additional skills adopt sub-agent composition, each would need its own copy
  of the guardrail instruction.

#### Suggestions

- 🔵 **Usability**: Add edge case for placeholder resolution failure
  **Location**: Testing Strategy: Edge Cases
  The edge cases section does not cover the failure mode where `{tmp directory}`
  is not resolved. Testing for literal `{tmp directory}` in created paths would
  strengthen confidence.

- 🔵 **Usability**: Consider splitting the dense IMPORTANT instruction
  **Location**: Phase 2: Changes Required, Section 1
  The proposed paragraph combines three distinct concerns. Separate callouts
  for general substitution and sub-agent composition would follow progressive
  disclosure.

- 🔵 **Architecture**: No mechanism to detect regression of /tmp hardcoding
  **Location**: Desired End State
  The grep-based verification is one-time. A CI lint could prevent
  reintroduction.

- 🔵 **Standards**: Line number references may shift after Phase 1 Change 1
  **Location**: Phase 1, Changes 2-3
  Adding the preprocessor line after line 16 shifts subsequent line numbers.
  The plan does not note this for implementers following changes sequentially.

### Strengths

- ✅ Clean phase separation — each phase modifies a single skill file with no
  cross-dependencies, making changes independently shippable and verifiable
- ✅ Follows established preprocessor precedent from `init/SKILL.md` rather
  than inventing new patterns
- ✅ Explicit "What We're NOT Doing" section demonstrates scope discipline and
  awareness of architectural tradeoffs
- ✅ Sub-agent prompt composition risk correctly identified as highest-impact
  failure mode and given dedicated mitigation
- ✅ Concrete, automatable success criteria with both grep checks and manual
  verification steps
- ✅ Fixes a real operational safety issue — `/tmp` risks OS cleanup and
  cross-process collisions

### Recommended Changes

1. **Add `mkdir -p {tmp directory}` step to describe-pr** (addresses: Missing
   mkdir -p finding)
   In Phase 1, add a step before writing the temporary body file that ensures
   the directory exists: `mkdir -p {tmp directory}`. This mirrors `review-pr`
   Step 1.2 and prevents failures for users who haven't run `/init`.

2. **Add a brief substitution instruction to describe-pr** (addresses:
   Asymmetric hardening finding)
   After the `**Tmp directory**:` preprocessor line in describe-pr, add a
   one-sentence substitution reminder consistent with Phase 2's approach
   and the `init` skill's line 125 precedent. This closes the gap between
   the two skills.

3. **Use `**IMPORTANT**:` format without em-dash sub-label** (addresses:
   Standards formatting finding)
   Change `**IMPORTANT — path substitution**:` to `**IMPORTANT**:` to match
   the existing convention across all skill files.

4. **Add a placeholder resolution failure edge case** (addresses: Missing
   edge case finding)
   Add a testing step that verifies no literal `{tmp directory}` strings
   appear in created paths or sub-agent prompts.

5. **Note line number shift after Change 1** (addresses: Line number
   references finding)
   Add a brief note that line numbers in Changes 2-3 refer to the original
   file before the preprocessor line is added.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-scoped and architecturally sound, correctly
identifying the two-hop resolution chain as the key failure mode and proposing
a minimal, convention-consistent fix. The changes are appropriately bounded to
two skill files with no structural impact on the broader system. One minor
architectural concern exists around the plan's reliance on natural-language
guardrails for a systemic placeholder-resolution problem, but the plan
explicitly acknowledges the decision not to pursue preprocessor-level fixes
and the approach is pragmatic for the current system.

**Strengths**:
- Clean separation of the two phases — each modifies a single skill file with
  no cross-file dependencies, making the changes independently shippable and
  verifiable
- Follows established architectural precedent (init/SKILL.md lines 18-30 and
  125) rather than inventing a new pattern, maintaining consistency across the
  skill catalogue
- The "What We're NOT Doing" section explicitly acknowledges architectural
  tradeoffs — particularly the decision not to extend the preprocessor —
  showing awareness of the boundary between this fix and a systemic solution
- The sub-agent prompt composition risk is correctly identified as the
  highest-impact failure mode and receives a dedicated, contextually-placed
  mitigation

**Findings**:

1. **Minor** (confidence: medium) — Natural-language guardrails address a
   systemic issue at a single site
   **Location**: Phase 2: Harden review-pr Template Variable Resolution
   The plan introduces explicit substitution instructions in
   `review-pr/SKILL.md` to mitigate the two-hop placeholder resolution
   failure. This is an instructional guardrail — it relies on LLM attention
   to prose rather than a structural mechanism. The bold-label-to-placeholder
   convention is used across all skills, so this same class of failure could
   recur in any future skill that composes sub-agent prompts.

2. **Suggestion** (confidence: medium) — Placement of Tmp directory
   preprocessor line relative to existing labels
   **Location**: Phase 1: Fix describe-pr Hardcoded /tmp, Change 1
   In the `init` skill, all path preprocessor labels are grouped together
   in a dedicated "Path Resolution" section (lines 18-30). In `describe-pr`,
   there is no such section heading.

3. **Suggestion** (confidence: low) — No mechanism to detect regression of
   the /tmp hardcoding pattern
   **Location**: Desired End State
   The grep-based automated verification is a one-time check. No ongoing
   mechanism prevents reintroduction of hardcoded `/tmp` paths.

### Correctness

**Summary**: The plan is logically sound for its core objective of replacing
hardcoded /tmp paths with configured tmp directory placeholders. The two-phase
approach correctly separates the bug fix from the instructional hardening. One
edge case gap exists: describe-pr lacks a directory creation step, which could
cause failures when meta/tmp does not yet exist.

**Strengths**:
- Phase 1 correctly mirrors the existing preprocessor pattern already used
  for PRs directory in the same file, reducing the risk of introducing a
  novel mechanism
- Phase 2 places the substitution instruction immediately after the
  bold-label definitions, which is the optimal position for LLM attention
  and follows the init skill precedent
- The plan's verification criteria are concrete and automatable, with both
  grep-based automated checks and manual invocation steps

**Findings**:

1. **Major** (confidence: high) — Missing mkdir -p for tmp directory in
   describe-pr
   **Location**: Phase 1: Changes Required, Section 2-3
   The plan replaces `/tmp/pr-body-{number}.md` with
   `{tmp directory}/pr-body-{number}.md`, but `/tmp` is guaranteed to exist
   while `meta/tmp` is not. If a user runs `/describe-pr` without first
   running `/init`, the write will fail.

2. **Minor** (confidence: medium) — Reminder instruction does not cover all
   path placeholders used in the sub-agent prompt
   **Location**: Phase 2: Changes Required, Section 2
   The Phase 2 reminder mentions `{pr reviews directory}` which does not
   appear in the sub-agent template. The mention is harmless but slightly
   misleading.

3. **Minor** (confidence: medium) — No explicit substitution instruction
   added to describe-pr unlike review-pr
   **Location**: Phase 1: Changes Required, Section 1
   Phase 2 adds explicit instructions to review-pr but Phase 1 adds none to
   describe-pr. The inconsistency means describe-pr does not benefit from the
   same hardening.

### Code Quality

**Summary**: The plan proposes a well-scoped, minimal fix to two skill files
with clear precedent from the existing codebase. The changes are simple text
substitutions and instructional additions that follow established patterns,
keeping complexity proportional to the problem.

**Strengths**:
- Follows DRY principle by reusing the exact same preprocessor pattern
  already established in review-pr and init
- Scope is tightly bounded — two files, no new abstractions, no new tools
- Explicitly documents what it is NOT doing, demonstrating YAGNI discipline
- Phase 1 and Phase 2 are independent, supporting incremental confidence
- Success criteria include both automated and manual verification steps

**Findings**:

1. **Minor** (confidence: medium) — Duplicated instructional text creates
   maintenance burden
   **Location**: Phase 2: Changes Required, Section 1 and 2
   Two separate instruction blocks convey the same substitution message. If
   placeholders change, both must be updated.

2. **Suggestion** (confidence: medium) — Implicit placeholder convention
   remains undocumented in describe-pr
   **Location**: Phase 1: Changes Required, Section 2 and 3
   Phase 1 adds placeholders to describe-pr without the explicit substitution
   instruction that Phase 2 adds to review-pr.

3. **Suggestion** (confidence: low) — Step numbering in plan may not match
   file nesting
   **Location**: Phase 1: Changes Required, Section 3
   The plan references steps 3-5 but these are sub-steps within step 9
   ("Update the PR") in the actual file.

### Standards

**Summary**: The plan follows established project conventions well,
particularly in how it adds the preprocessor bold-label line and references
existing precedent from the init skill. One minor naming convention
inconsistency exists in the proposed instruction format.

**Strengths**:
- The proposed **Tmp directory** preprocessor line exactly matches the syntax
  and placement pattern used in review-pr and init
- Correctly identifies and follows the bold-label-to-placeholder convention
  used uniformly across all skills
- Phase ordering and scope boundaries follow plan structure conventions

**Findings**:

1. **Minor** (confidence: medium) — Instruction formatting departs from
   existing IMPORTANT pattern
   **Location**: Phase 2, Change 1
   The `**IMPORTANT — path substitution**:` format uses an em-dash sub-label
   style not found elsewhere. Existing skills use `**IMPORTANT**:`.

2. **Suggestion** (confidence: medium) — Consider adding substitution
   instruction to describe-pr
   **Location**: Phase 2, Change 1
   The init skill's line 125 precedent shows substitution reminders are used
   even in single-agent contexts.

3. **Suggestion** (confidence: low) — Line number references may shift after
   Change 1
   **Location**: Phase 1, Change 3
   Adding a preprocessor line after line 16 shifts subsequent line numbers.
   The plan does not note this.

### Usability

**Summary**: The plan addresses a genuine developer experience issue —
hardcoded /tmp paths silently produce wrong behavior. The fix follows
established patterns and references concrete precedent. The main usability
concern is that Phase 2 relies on natural-language instructions rather than
a structural guarantee.

**Strengths**:
- Follows established patterns exactly, so developers who understand one path
  resolution will immediately understand the other
- Explicitly identifies the sub-agent prompt composition boundary as a failure
  point
- Verification steps are concrete and actionable
- Scope boundaries prevent creep into preprocessor redesign

**Findings**:

1. **Major** (confidence: medium) — LLM instruction-based substitution is a
   soft guarantee with no failure signal
   **Location**: Phase 2: Changes Required, Section 1 and 2
   If the LLM fails to follow the instructions, artefacts silently land in
   `/tmp` or literal `{tmp directory}` appears in paths, producing confusing
   errors with no root cause indication.

2. **Minor** (confidence: high) — Consistency gap: describe-pr lacks the
   explicit substitution instruction
   **Location**: Phase 1: Changes Required, Section 2 and 3
   Phase 1 adds placeholders without the guardrail that Phase 2 adds to
   review-pr.

3. **Suggestion** (confidence: medium) — Consider splitting the dense
   IMPORTANT instruction
   **Location**: Phase 2: Changes Required, Section 1
   The proposed paragraph combines three distinct concerns. Separate callouts
   would follow progressive disclosure.

4. **Suggestion** (confidence: low) — No edge case for placeholder resolution
   failure
   **Location**: Testing Strategy: Edge Cases
   Testing for literal `{tmp directory}` in paths would strengthen confidence.

### Safety

**Summary**: The plan addresses a genuine data safety issue — files being
written to a system-wide `/tmp` directory instead of a project-scoped
`meta/tmp` directory. The changes are low-risk text edits with no destructive
operations.

**Strengths**:
- Fixes a real operational safety issue: `/tmp` risks collisions with other
  processes and unpredictable OS cleanup
- Phases are independent, limiting blast radius
- Preserves the existing cleanup step for temporary files
- Verification includes both automated and manual checks

**Findings**:

1. **Minor** (confidence: medium) — No safeguard against missing tmp
   directory in describe-pr
   **Location**: Phase 1, Change 2
   `describe-pr` has no `mkdir -p` step. Users who haven't run `/init` would
   encounter a non-obvious failure.

2. **Minor** (confidence: medium) — Instructional guardrails are best-effort
   with no fail-safe fallback
   **Location**: Phase 2: Harden review-pr Template Variable Resolution
   The fix is probabilistic rather than deterministic. This is an acceptable
   tradeoff for the current scope but worth noting as a known limitation.

## Re-Review (Pass 2) — 2026-04-08

**Verdict:** APPROVE

### Previously Identified Issues

- ✅ **Correctness**: Missing `mkdir -p` for tmp directory in describe-pr — Resolved (Change 2 added with `mkdir -p {tmp directory}`)
- ✅ **Usability**: LLM instruction-based substitution has no failure signal — Resolved (placeholder resolution failure edge case added; IMPORTANT block split into two focused callouts)
- ✅ **Correctness + Code Quality + Standards + Usability**: No explicit substitution instruction in describe-pr — Resolved (IMPORTANT substitution instruction added to Phase 1 Change 1)
- ✅ **Standards**: Instruction formatting departs from IMPORTANT convention — Resolved (now uses `**IMPORTANT**:` format)
- ✅ **Code Quality**: Duplicated instructional text — Resolved (split into two distinct callouts with separate concerns)
- ✅ **Architecture**: Natural-language guardrails at single site — Resolved (both skills now receive substitution instructions)
- ✅ **Usability**: No edge case for placeholder resolution failure — Resolved (edge case added)
- ✅ **Usability**: Dense IMPORTANT instruction — Resolved (split into two focused callouts)
- ✅ **Standards**: Line number references may shift — Resolved (note added about line shifts)
- ✅ **Architecture**: No regression detection mechanism — Still present (suggestion, accepted as out of scope)
- ✅ **Correctness**: Reminder mentions `{pr reviews directory}` not in sub-agent template — Resolved (scoped to `{tmp directory}` only)
- ✅ **Safety**: No safeguard against missing tmp directory — Resolved (mkdir -p added)
- ✅ **Safety**: Instructional guardrails are best-effort — Still present (accepted tradeoff, documented)

### New Issues Introduced

- 🔵 **Correctness + Code Quality + Standards + Usability**: Phase 2 automated verification grep patterns did not match proposed text — Fixed (updated grep patterns to match actual instruction text)
- 🔵 **Standards**: Line shift note understated number of lines added — Fixed (note now says "several lines" instead of "1")

### Assessment

All major and minor findings from the initial review have been addressed. The two new minor issues introduced by the edits (mismatched grep patterns and understated line shift) were fixed immediately. The plan is now ready for implementation. Only two low-priority suggestions remain open (CI regression detection and the inherent limitation of instruction-based guardrails), both explicitly accepted as out of scope.
