---
date: "2026-04-26T00:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-26-cleanup-remaining-ticket-references.md"
review_number: 1
verdict: APPROVE
lenses: [correctness, safety, test-coverage, code-quality, standards, documentation]
review_pass: 2
status: complete
---

## Plan Review: Cleanup Remaining Ticket References

**Verdict:** REVISE

The plan is well-researched, clearly scoped, and correctly orders changes by user-facing impact. Production code is clean and the phasing rationale is sound. However, four major issues prevent it from being ready for implementation as written: the final verification grep cannot pass because two categories of missed references are not covered by any phase, one enumerated call-site list is missing a real call site that would leave a broken bash function call after implementation, and the `make_ticket` call-site line numbers are materially inaccurate — listing 15 sites at wrong lines when 10 real sites exist at different offsets.

### Cross-Cutting Themes

- **Verification grep will fail after complete implementation** (flagged by: Correctness, Standards) — Both a stale comment on line 100 of `test-work-item-scripts.sh` (`"ticket number space exhausted"`) and two `ticket_revise_severity` references in `skills/work/review-work-item/SKILL.md` (lines 208 and 246) sit inside the scope of the final completeness grep but are not addressed by any phase. Running the plan's own success-criteria check after four phases complete will return hits and report failure.

- **Incomplete call-site enumeration creates partial-rename risk** (flagged by: Correctness, Safety, Test Coverage) — The enumerated line lists for `make_ticket` (15 lines listed, 10 real sites) and `make_tagged_ticket` (9 lines listed, 10 real sites — line 520 omitted) are unreliable as navigation guides. If an implementer uses the line numbers directly rather than the "replace all occurrences" instruction, the test suite will fail with an undefined-function error or a residual `ticket` reference in the grep.

### Findings

#### Major

- 🟡 **Correctness + Safety**: Missing `make_tagged_ticket` call site at line 520
  **Location**: Phase 3 — `skills/work/scripts/test-work-item-scripts.sh`, `make_tagged_ticket` call sites
  The plan enumerates call sites to rename as lines 463, 470, 477, 484, 491, 498, 577, 584, 591, but omits line 520 (`make_tagged_ticket "$REPO" "tags: []"` for Test 8: Add to empty array). After applying the enumerated renames, line 520 retains the old function name and calls a non-existent function, causing a bash error when Test 8 runs. The verification grep would also catch `tagged_ticket` and report failure.

- 🟡 **Correctness + Safety + Test Coverage**: `make_ticket` call-site line numbers are inaccurate
  **Location**: Phase 3 — `skills/work/scripts/test-work-item-scripts.sh`, `make_ticket` call sites
  The plan lists 15 call sites at lines 307, 314, 321, 328, 335, 342, 350, 358, 364, 374, 380, 386, 392, 407, 413. The actual call sites are 10, at lines 307, 314, 321, 328, 335, 341, 348, 387, 407, 413. Lines 350, 358, 364, 374, 380, 386, 392 contain unrelated code. An implementer using the line list directly would make no-op edits at those lines and miss real sites at 341, 348, and 387.

- 🟡 **Correctness**: Phase 3 verification grep fails due to uncovered line 100 comment
  **Location**: Phase 3 success criteria / Testing Strategy
  After all Phase 3 changes, line 100 of `test-work-item-scripts.sh` still reads `# Test 10: Highest 9999 → exits 1 with "ticket number space exhausted" error`. The actual script already emits `"work item number space exhausted"` (correctly updated), so this comment misquotes the current error message and is a genuine missed reference. The Phase 3 grep (`grep -rni ticket skills/work/scripts/`) will match this line and fail even after a correct implementation of all enumerated changes.

- 🟡 **Standards**: Two `ticket_revise_severity` references in `review-work-item/SKILL.md` cause final verification grep to fail
  **Location**: Phase 4 — `skills/work/review-work-item/SKILL.md`, and Desired End State grep
  The plan's Desired End State grep explicitly includes `skills/work/review-work-item/SKILL.md`, and Phase 4 schedules a fix for line 104 (`BUILTIN_TICKET_LENSES`). However, `ticket_revise_severity` appears at lines 208 and 246 of the same file. These are configuration key references in prose, not comments, and are not covered by any phase. The final completeness check will return two hits and fail.

#### Minor

- 🔵 **Test Coverage**: Fixture frontmatter key change alters test input without explicit acknowledgement
  **Location**: Phase 3 — `test-work-item-scripts.sh` lines 218 and 268
  Changing `ticket_id: 0001` to `work_item_id: 0001` in the inline printf fixtures is a semantic change to test inputs, not just a cosmetic rename. The existing assertions only check the `status` output, so if `work-item-read-status.sh` were key-sensitive the tests would still pass. The plan does not confirm the script is key-agnostic; a brief explicit note or a check of that script's behaviour is warranted.

- 🔵 **Safety**: Phase 3 grep scope excludes Phase 4 target files
  **Location**: Phase 3 success criteria
  The Phase 3 grep covers `scripts/` and `skills/work/scripts/` only. If phases are applied out of order or Phase 4 is skipped, the Phase 3 gate passes green while Phase 4 targets remain dirty. Only the final Phase 4 corpus grep catches everything — the plan should note this explicitly.

- 🔵 **Safety**: Phase 1 → Phase 2 gap leaves agent emitting stale paths
  **Location**: Phase 2 — `agents/documents-locator.md`
  Any agent invocation between completing Phase 1 and Phase 2 returns `meta/tickets/` paths that point to a directory that no longer exists. Committing Phase 1 and Phase 2 together would eliminate this window.

- 🔵 **Standards**: Directory tree comment uses inconsistent form (`# Work item files` vs sibling pattern)
  **Location**: Phase 2 — `agents/documents-locator.md` lines 54–55
  The proposed replacement `├── work/         # Work item files` adds the word "files" not present in any sibling entry in the same tree (e.g., `# Ticket documentation` → `# Research documents` form). This introduces a style inconsistency within the tree.

- 🔵 **Correctness**: Capitalisation inconsistency in `documents-locator.md` after changes
  **Location**: Phase 2 — `agents/documents-locator.md` line 25 vs line 74
  The line 25 replacement uses `Work items` (sentence case) while the line 74 heading replacement uses `### Work Items` (title case). These are different capitalisation choices for the same concept in the same file.

- 🔵 **Documentation**: ADR reference path does not resolve
  **Location**: References section
  The References section cites `meta/decisions/ADR-0022-work-item-as-canonical-term.md`, but no file exists at that path. The ADR may use a different path or may not yet exist.

- 🔵 **Correctness**: Line number discrepancy between plan and research for `0026` change
  **Location**: Phase 4 — `meta/work/0026-init-skill-for-repo-bootstrap.md`
  The plan specifies line 107 but the research document cites line 108 for the same reference. The text is unique enough to locate visually but indicates the line numbers were not re-verified.

#### Suggestions

- 🔵 **Test Coverage**: Manual template verification not mirrored by an automated test
  **Location**: Phase 1 success criteria — Manual Verification
  The "new plan carries `work-item:` in frontmatter" check is the only manual step. A lightweight automated test that asserts `work-item:` is present and `ticket:` is absent in `templates/plan.md` would make this invariant machine-checkable.

- 🔵 **Documentation**: Example output paths in `documents-locator.md` use `eng-XXXX` form inconsistent with updated naming hint
  **Location**: Phase 2 — `agents/documents-locator.md` lines 75–76
  The plan updates line 121 to say files are named `NNNN-title.md`, but leaves the example paths as `meta/work/eng-1234.md` and `meta/work/eng-1235.md`. After the plan is applied the naming hint and example paths will be internally inconsistent within the same file.

### Strengths

- ✅ The phasing by user-facing impact is correct — template and agent definition changes (Phases 1–2) carry ongoing production cost while stale and are rightly prioritised first.
- ✅ The coordinated-rename requirement for `TICKET_LENSES`/`_is_ticket_lens` is correctly identified: a partial rename produces a bash error surfaced immediately by the test suite.
- ✅ The two-layer verification strategy (grep check + test suite) is sound; grep catches textual misses, tests catch functional regressions.
- ✅ The "What We're NOT Doing" section is precise and well-justified — migration scripts, CHANGELOG, ADR body text, and settings.local.json are all correctly excluded with clear reasoning.
- ✅ The plan correctly identifies the inline `printf` fixtures at lines 218/268 (`ticket_id:`) as inconsistent with the `make_ticket` heredoc (already using `work_item_id:`) and includes them for cleanup.
- ✅ The Testing Strategy section explicitly warns that Phase 3 renames must be applied atomically before running tests — the right guidance for the highest-risk step.
- ✅ Each phase provides a discrete grep and test-run checkpoint, enabling partial completion to be verified incrementally.

### Recommended Changes

1. **Add line 520 to the `make_tagged_ticket` call-site list** (addresses: missing call site at line 520)
   Add `520` to the enumerated call sites for `make_tagged_ticket`, or add a bold note that the "replace all occurrences" instruction is authoritative and the line list is illustrative only.

2. **Correct the `make_ticket` call-site line list** (addresses: inaccurate line numbers)
   Replace the 15-line list with the 10 actual sites (307, 314, 321, 328, 335, 341, 348, 387, 407, 413), or make the "replace all occurrences" instruction the primary guide and remove the potentially misleading line enumeration.

3. **Add a Phase 3 change for line 100 of `test-work-item-scripts.sh`** (addresses: verification grep failure)
   Rename `"ticket number space exhausted"` → `"work item number space exhausted"` in the comment at line 100 to match the current error message in the script under test.

4. **Add Phase 4 changes for `ticket_revise_severity` in `skills/work/review-work-item/SKILL.md`** (addresses: final verification grep failure)
   Add entries for lines 208 and 246 renaming `ticket_revise_severity` to the established convention (e.g., `work_item_revise_severity`), or explicitly move these to "What We're NOT Doing" with a justification if the key name is intentionally stable.

5. **Update example output paths in `documents-locator.md`** (addresses: naming hint inconsistency)
   Change `meta/work/eng-1234.md` and `meta/work/eng-1235.md` at lines 75–76 to `NNNN-title` form (e.g., `meta/work/0001-implement-rate-limiting.md`) to match the naming hint update at line 121.

6. **Fix or remove the ADR reference** (addresses: broken reference path)
   Verify the correct path for ADR-0022 and update the References section, or remove the reference if the ADR does not yet exist.

7. **Align capitalisation in `documents-locator.md`** (addresses: capitalisation inconsistency)
   Reconcile `Work items` (line 25 replacement) and `Work Items` (line 74 heading replacement) — title case for the heading, sentence case for the bullet is natural English and may be intentional; if so, the line 25 change is already correct and only the heading needs `### Work Items`.

---
*Review generated by /review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan is logically sound and its scope is well-defined. The coordinated rename strategy in Phase 3 is correctly identified as requiring atomic application, and the verification greps are appropriate. Two correctness issues stand out: one enumerated call-site list omits a real call site (leaving a residual reference after implementation), and the plan's own verification grep would fail because one additional 'ticket' reference in the same file is not included in any phase's changes.

**Strengths**:
- The plan explicitly identifies that variable/function renames must be atomic — renaming TICKET_LENSES and _is_ticket_lens without updating all call sites would produce a bash error caught immediately by the test suite, so the atomicity requirement is stated correctly.
- The two-layer verification strategy (grep check + test suite) is sound: the grep catches textual misses and the test suite catches functional regressions from partial renames.
- The phasing by user-facing impact is correct — template changes (Phase 1) and agent definition changes (Phase 2) have ongoing production cost while stale, and are rightly prioritised over comment-only fixes.
- The out-of-scope exclusions are correctly justified: migration scripts intentionally reference old names as input targets, CHANGELOG is historical record, and ADR Ticket headings are generic English prose outside the machine-readable rename scope.
- The plan correctly identifies that the fixture frontmatter at lines 218 and 268 (ticket_id:) are NOT migration test fixtures but regular work-item helper tests that should use work_item_id.

**Findings**:

**[Major/High]** Missing `make_tagged_ticket` call site at line 520 leaves a residual reference
Location: Phase 3, Changes Required: skills/work/scripts/test-work-item-scripts.sh — make_tagged_ticket call sites
The plan enumerates `make_tagged_ticket` call sites to rename as lines 463, 470, 477, 484, 491, 498, 577, 584, 591, but omits line 520 which also contains `make_tagged_ticket "$REPO" "tags: []"` (Test 8: Add to empty array). The research document carries the same omission. After applying all enumerated renames, line 520 would retain the old function name and call a non-existent function, causing a bash error when that test runs. The verification grep would also catch this since 'tagged_ticket' matches '-i ticket'.

**[Major/High]** Enumerated `make_ticket` call-site line numbers do not match actual call locations
Location: Phase 3, Changes Required: skills/work/scripts/test-work-item-scripts.sh — make_ticket call site lines
The plan lists 15 `make_ticket` call sites at lines 307, 314, 321, 328, 335, 342, 350, 358, 364, 374, 380, 386, 392, 407, 413. The actual `make_ticket` calls in the file are at lines 307, 314, 321, 328, 335, 341, 348, 387, 407, 413 — 10 sites, not 15. Lines 350, 358, 364, 374, 380, 386, 392 do not contain `make_ticket` calls; they are `assert_eq`, `REPO=$(setup_repo)`, `cat >`, and `assert_exit_code` statements. An implementer navigating to the listed line numbers would find unrelated code and could miss the real call sites at 341, 348, and 387.

**[Major/High]** Phase 3 verification grep will fail due to uncovered 'ticket number space exhausted' comment
Location: Phase 3 success criteria / Testing Strategy
After applying all Phase 3 changes, one `ticket` reference remains in `skills/work/scripts/test-work-item-scripts.sh` that the plan does not cover: line 100 reads `# Test 10: Highest 9999 → exits 1 with "ticket number space exhausted" error`. The actual script already emits `"work item number space exhausted"` (correctly updated), so this comment misquotes the current error message. The Phase 3 success-criteria grep `grep -rni ticket skills/work/scripts/` would match this line and report a non-zero exit.

**[Minor/High]** Phase 2 diff for line 25 capitalisation inconsistency
Location: Phase 2, Changes Required: agents/documents-locator.md line 25 diff
The line 25 replacement uses `Work items` (sentence case) while the line 74 heading replacement uses `### Work Items` (title case). These are different capitalisation choices for the same concept in the same file, producing an inconsistency after the plan is applied.

**[Minor/Medium]** Line number discrepancy between plan and research for 0026 change
Location: Phase 4, Changes Required: meta/work/0026-init-skill-for-repo-bootstrap.md line number
The plan specifies line 107 but the research document cites line 108. The text is unique enough to locate visually, but indicates the line numbers were not re-verified.

---

### Safety

**Summary**: The plan makes purely textual terminology changes across test scripts, templates, and documentation — no production data is modified and no destructive operations are involved. The primary safety risk is the coordinated multi-site rename in Phase 3, where a partially applied rename leaves bash scripts in a broken state until the test suite runs. The plan explicitly acknowledges this and instructs implementers to verify atomicity before running tests, which is an appropriate safeguard.

**Strengths**:
- Every phase ends with 'mise run test', meaning broken state from a partial rename is surfaced immediately rather than silently persisting.
- The plan explicitly warns that Phase 3 renames are coordinated and must be applied atomically, reducing the probability of a partial rename being committed.
- The 'What We're NOT Doing' section explicitly scopes out historical content (CHANGELOG, migration scripts, ADR body text) and explains the rationale — preventing accidental over-application of the cleanup.
- The grep verification commands in each phase's success criteria are precise and narrow, targeting only the files changed in that phase.
- Production code is confirmed clean at the outset; all changes are confined to test infrastructure, templates, and documentation.

**Findings**:

**[Major/High]** Partial rename of make_ticket call sites leaves test harness silently broken
Location: Phase 3: Functional Script Renames — test-work-item-scripts.sh section
The plan lists specific line numbers for call sites but does not verify this list is exhaustive. Bash with `set -euo pipefail` will halt the script on the first call to an undefined function, so a single missed call site causes every subsequent test in the file to be silently skipped rather than failing explicitly — the test counter will report fewer tests, but the exit code may still be 0 if the surviving tests pass. Before applying the renames, `grep -n 'make_ticket\|make_tagged_ticket' skills/work/scripts/test-work-item-scripts.sh` should be run to verify the ground-truth count.

**[Minor/High]** Phase 3 grep scope excludes files cleaned in Phase 4
Location: Phase 3: Success Criteria — Automated Verification
The Phase 3 success criteria grep covers `scripts/` and `skills/work/scripts/` only, not the Phase 4 targets. If phases are applied out of order or Phase 4 is skipped, the Phase 3 gate passes green while Phase 4 targets remain dirty. Only the final Phase 4 corpus grep catches everything; the plan should note this.

**[Minor/Medium]** Stale agent example output actively misdirects until Phase 2 is applied
Location: Phase 2: Agent Definition — agents/documents-locator.md
Any agent invocation between completing Phase 1 and Phase 2 returns `meta/tickets/` paths pointing to a directory that no longer exists. Committing Phase 1 and Phase 2 together would eliminate this window.

---

### Test Coverage

**Summary**: The plan is a cosmetic-plus-functional rename across test scripts, templates, and documentation — no new code paths are introduced. The existing test suite is relied on as the sole regression gate, which is appropriate, but the plan's Testing Strategy section does not acknowledge the semantic risk of the fixture-content change (ticket_id: → work_item_id: in inline printf fixtures).

**Strengths**:
- Every phase ends with an explicit 'Tests pass: mise run test' checkpoint, providing continuous regression protection after each incremental change.
- The plan identifies the coordinated multi-site rename in Phase 3 as atomic and explicitly warns that a partial rename will produce a bash error caught by the test suite.
- The grep-based completeness check at the end of Phase 4 acts as a second independent verification layer alongside the test suite.
- The plan correctly excludes intentionally-historical references from scope, avoiding spurious test churn.
- The eval assertion rename in evals.json is handled as a named field change only, preserving all existing assertion types and check expressions intact.

**Findings**:

**[Major/High]** Fixture frontmatter key change alters test input semantics without explicit verification
Location: Phase 3: Functional Script Renames — test-work-item-scripts.sh lines 218 and 268
The plan changes the inline printf fixtures from `ticket_id: 0001` to `work_item_id: 0001`. These fixtures are the actual file content fed to `work-item-read-status.sh` in tests 5 and 10. This is a behavioural change to the test input — not merely a cosmetic rename. If `work-item-read-status.sh` validates or cares about the presence of `work_item_id:`, a silent regression could slip through because the test assertions only check the `status` output. The plan does not confirm the script is key-agnostic.

**[Minor/High]** Plan provides no exhaustive count verification step for call sites
Location: Phase 3: Functional Script Renames — test-work-item-scripts.sh, make_ticket call site inventory
The plan lists specific line numbers but the success criteria only runs a grep without a step confirming the rename count matches. A pre-rename `grep -c 'make_ticket\b' skills/work/scripts/test-work-item-scripts.sh` would verify expected occurrence count before renaming.

**[Minor/Medium]** Lines 512 and 536 fixture headings not linked to a specific test path
Location: Phase 3: Functional Script Renames — test-work-item-scripts.sh lines 512 and 536
The plan targets these H1 headings but does not clarify whether they are inside `make_tagged_work_item` call coverage or in independent inline fixtures. A reviewer implementing this plan cannot confirm from the plan alone which test cases use these fixtures.

**[Suggestion/Medium]** Manual verification step for templates not mirrored by automated test
Location: Testing Strategy section
The "new plan carries `work-item:` in frontmatter" check is the only manual step. A lightweight automated test asserting `work-item:` is present and `ticket:` absent in `templates/plan.md` would make this invariant machine-checkable.

---

### Code Quality

**Summary**: The plan is a well-scoped, purely terminological cleanup with no design changes. The coordinated-rename requirement for the bash variable and function in Phase 3 is correctly identified and handled. From a code quality perspective the plan is sound; the only concerns are pre-existing inconsistencies within the test file that the plan itself is correcting.

**Strengths**:
- Phase 3 correctly identifies the coordinated-rename requirement: TICKET_LENSES and _is_ticket_lens must be renamed atomically with all their call sites or the script will fail with an undefined-function error.
- The plan distinguishes functional changes from cosmetic changes — a pragmatic and useful distinction for a developer executing the changes.
- Scoping exclusions are well-reasoned with explicit justifications, making it clear what is intentional.
- The inline printf fixtures at lines 218 and 268 using ticket_id: while the make_ticket heredoc already uses work_item_id: are correctly identified as an internal inconsistency.
- Each phase has its own automated grep verification step and test run.

**Findings**:

**[Minor/High]** make_ticket fixture mixes old and new terminology within a single function body
Location: Phase 3: Functional Script Renames — scripts/test-lens-structure.sh
The `make_ticket` helper already uses `work_item_id:` in the frontmatter but retains `# 0001: Test Ticket` as the H1 heading. The plan correctly targets both the comment and the H1; no additional action is needed beyond executing Phase 3 as written.

**[Minor/High]** Inline printf fixtures use ticket_id: while heredoc fixtures use work_item_id:
Location: Phase 3: Functional Script Renames — test-work-item-scripts.sh lines 218 and 268
A future developer maintaining these tests will encounter two different frontmatter key names for what is conceptually the same field. The plan correctly targets both sites; ensure they are not missed during execution.

**[Suggestion/Medium]** Agent categorisation list still implies tickets/ directory after rename
Location: Phase 2: Agent Definition — agents/documents-locator.md
The plan lists all five sub-changes for Phase 2 as a single phase but does not call out that a partial update will still yield grep hits. Consider an explicit note that all five sub-changes in documents-locator.md must be made together before running the Phase 2 grep verification.

**[Suggestion/Low]** Eval assertion name rename is correct; description field is already accurate
Location: Phase 4: Minor Fixes — skills/work/update-work-item/evals/evals.json
The rename to `handles_legacy_work_item` resolves the terminology mismatch. No additional change needed.

---

### Standards

**Summary**: The plan is well-structured and correctly identifies the naming convention established by ADR-0022. The proposed renames are internally consistent and follow established patterns. However, the plan's own final verification grep targets `skills/work/review-work-item/SKILL.md` but only schedules one fix there, leaving two occurrences of `ticket_revise_severity` at lines 208 and 246 that would cause the completeness check to fail.

**Strengths**:
- All proposed variable and function renames follow the established snake_case convention consistent with surrounding code.
- The frontmatter key rename `ticket:` → `work-item:` correctly matches the hyphenated form already in use.
- The eval assertion rename follows the snake_case naming pattern used by all other assertion names in evals.json.
- Phase 3's treatment of coordinated renames reflects correct understanding of bash scoping.
- The directory structure example fix in agents/documents-locator.md aligns with the actual meta/work/ convention.
- The plan's own frontmatter already uses `work-item:` rather than `ticket:`.

**Findings**:

**[Major/High]** Two `ticket_revise_severity` references in review-work-item/SKILL.md are out of plan scope but inside the grep fence
Location: Phase 4: Minor Fixes — skills/work/review-work-item/SKILL.md, and Desired End State grep
The plan's Desired End State grep includes `skills/work/review-work-item/SKILL.md`, but Phase 4 only schedules a fix for line 104. The same file contains `ticket_revise_severity` at lines 208 and 246. After all four phases, the final verification grep will still return two hits in this file.

**[Minor/High]** Directory tree comment uses `# Work item files` inconsistent with sibling pattern
Location: Phase 2: Agent Definition — agents/documents-locator.md lines 54–55
The proposed replacement `├── work/         # Work item files` adds "files" not present in any sibling entry in the tree. Sibling entries use the noun form (`# Ticket documentation`, `# Research documents`). The replacement should use `# Work item documentation` or `# Work items` to match.

**[Minor/Medium]** Script comment style inconsistency in test-hierarchy-format.sh is pre-existing
Location: Phase 4: Minor Fixes — scripts/test-hierarchy-format.sh
The comment fix is correct and low-risk. The inconsistency in path style relative to other script headers pre-exists and is not introduced by this plan.

---

### Documentation

**Summary**: The plan is a well-documented, self-contained cleanup guide with precise diffs and clear rationale for scope exclusions. One broken reference in the References section and an internal inconsistency introduced by updating the naming hint without updating the example paths are the only documentation gaps.

**Strengths**:
- The Overview and Current State Analysis give any implementer immediate orientation without reading the research document.
- The 'What We're NOT Doing' section explicitly justifies every exclusion, eliminating ambiguity about scope.
- Each phase includes both automated grep verification commands and, where relevant, a manual verification step.
- All diffs verified against the live codebase match current file contents for templates/plan.md, templates/pr-description.md, agents/documents-locator.md, and scripts/test-lens-structure.sh.
- The research document is referenced directly, establishing a clear audit trail, and the commit anchor (f03a7dfe) gives implementers a stable baseline for line-number validation.
- The Testing Strategy section provides a clear warning about the atomicity requirement for Phase 3's coordinated multi-site renames.

**Findings**:

**[Minor/High]** ADR reference path does not resolve
Location: References section
The References section cites `meta/decisions/ADR-0022-work-item-as-canonical-term.md`, but no file exists at that path in the repository. An implementer or reviewer who follows the reference to understand the canonical term decision will hit a dead link.

**[Minor/High]** Plan fix for agent output block is internally consistent (confirmation, no action)
Location: Phase 2: Agent Definition — agents/documents-locator.md
The plan's changes to the categorisation list (line 25) and the example output heading (line 74) are consistent after the rename. This is a confirmation that the plan is correct for these two sites; no action required.

**[Suggestion/Medium]** Example output paths in documents-locator.md use `eng-XXXX` form inconsistent with updated naming hint
Location: Phase 2: Agent Definition — agents/documents-locator.md lines 75–76
The plan updates line 121 to say files are named `NNNN-title.md`, but leaves the example paths as `meta/work/eng-1234.md` and `meta/work/eng-1235.md`. After the plan is applied, the naming hint and example paths will be internally inconsistent within the same file. Updating the example paths to `NNNN-title` form (e.g., `meta/work/0001-implement-rate-limiting.md`) would resolve this.

## Re-Review (Pass 2) — 2026-04-26

**Verdict:** APPROVE

### Previously Identified Issues

- 🟡 **Correctness + Safety**: Missing `make_tagged_ticket` call site at line 520 — **Resolved**. Line 520 added to the enumerated list; all 10 sites verified against the source file.
- 🟡 **Correctness + Safety + Test Coverage**: `make_ticket` call-site line numbers inaccurate — **Resolved**. List corrected to the 10 actual sites (307, 314, 321, 328, 335, 341, 348, 387, 407, 413); all verified.
- 🟡 **Correctness**: Phase 3 verification grep failing due to line 100 comment — **Resolved**. New change entry added to Phase 3 for the `"ticket number space exhausted"` comment.
- 🟡 **Standards**: Two `ticket_revise_severity` references in `review-work-item/SKILL.md` — **Resolved**. Phase 4 now includes diffs for lines 208 and 246; verified accurate against the source file.

### New Issues Introduced

- 🟡 **Correctness** (now resolved): The re-review found that the diffs for lines 66–67 and 74–75 in Phase 3 updated only the comment lines, leaving the `echo` lines on lines 67 and 75 with residual `ticket` text that would still be caught by the Phase 3 verification grep. Both diffs have been extended to cover the echo lines. This was fixed before closing the re-review.

### Minor Findings Accepted Without Change

- 🔵 **Test Coverage**: Test 10's comment correction (`"ticket number space exhausted"`) is not validated by any assertion on stderr. The test asserts exit code and stdout only; accepted as low-risk since the comment is cosmetic and the functional test remains valid.

### Assessment

All four major findings from Pass 1 are resolved. The one new issue surfaced during the re-review (echo lines on 67 and 75) was fixed immediately before closing. The plan is now internally consistent: call-site enumerations match source reality, the final verification grep will return zero results after all four phases, and the phase-by-phase test gates provide appropriate regression protection. The plan is ready for implementation.
