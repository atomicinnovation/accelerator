---
date: 2026-02-22T17:06:11+00:00
researcher: Toby Clemson
git_commit: n/a
branch: n/a
repository: .claude
topic: "Differences between review-pr and review-plan commands for alignment upgrade"
tags: [research, codebase, review-plan, review-pr, plan-reviewers, pr-reviewers, alignment]
status: complete
last_updated: 2026-02-22
last_updated_by: Toby Clemson
---

# Research: Differences Between review-pr and review-plan for Alignment Upgrade

**Date**: 2026-02-22T17:06:11+00:00
**Researcher**: Toby Clemson
**Repository**: .claude

## Research Question

The `review-plan` command and its `plan-*-reviewer` agents were created first.
The `review-pr` command and its `pr-*-reviewer` agents are more recent and have
evolved further — particularly with the addition of structured JSON output and
inline comment support. Analyse the differences so we can plan an upgrade to
make `review-plan` consistent with the newer patterns.

## Summary

The two command/agent sets share the same core architecture (multi-lens review
with parallel agent spawning) but diverge significantly in three areas:

1. **Agent output format**: PR agents output structured JSON; plan agents output
   free-form markdown
2. **Command synthesis and presentation**: PR command has a structured
   aggregation pipeline with deduplication, validation, and verdict logic;
   plan command has looser narrative synthesis
3. **Lens coverage**: PR has 6 lenses (adds Test Coverage); plan has 5 lenses
   (no Test Coverage equivalent)
4. **Post-review workflow**: PR command posts to GitHub API; plan command edits
   the plan file directly

## Detailed Findings

### 1. Agent Output Format (Most Significant Difference)

#### PR Agents (current — after inline comments upgrade)

All 6 PR agents output **structured JSON** with a defined schema:

```json
{
  "lens": "architecture",
  "summary": "2-3 sentence assessment",
  "strengths": ["..."],
  "comments": [
    {
      "path": "file.ts",
      "line": 42,
      "end_line": null,
      "side": "RIGHT",
      "severity": "critical",
      "confidence": "high",
      "lens": "architecture",
      "title": "Finding title",
      "body": "🔴 **Architecture**\n\n..."
    }
  ],
  "general_findings": [
    {
      "severity": "minor",
      "lens": "architecture",
      "title": "Cross-cutting finding",
      "body": "..."
    }
  ]
}
```

Key features:
- Machine-parseable output
- Separation of line-anchored `comments` vs `general_findings`
- Self-contained comment bodies with emoji severity prefixes
- Explicit `lens` field on every finding for attribution after merging
- `confidence` rating on each comment
- Detailed Field Reference, Multi-Line Comment API Mapping, Severity Emoji
  Prefixes, and Comment Body Format subsections

Each PR agent also has an explicit final analysis step: **"Anchor Findings to
Diff Locations"** (Step 5 or 6 depending on agent) that instructs the agent to
map each finding to a precise diff line.

#### Plan Agents (current — older pattern)

All 5 plan agents output **free-form markdown** following a template:

```markdown
## Architecture Review: [Plan Name]

### Summary
[2-3 sentence assessment]

### Findings

#### Critical
- **[Finding title]** (confidence: high/medium/low)
  **Location in plan**: [Section or line reference]
  **Issue**: [What the problem is]
  **Impact**: [Why it matters]
  **Suggestion**: [Concrete alternative]

#### Major
...

### Strengths
- [Decisions the plan gets right]
```

Key features:
- Human-readable but not machine-parseable
- No separation of anchored vs general findings
- Findings reference "Location in plan" (section/line) but format is informal
- No explicit `lens` field — attribution is implicit from the agent that produced it
- Lens-specific assessment sections (e.g., "Architectural Tradeoffs Identified",
  "Codebase Consistency") appear after findings

### 2. Command Orchestration Differences

#### review-pr Command (newer)

| Feature | Implementation |
|---------|---------------|
| **Input handling** | Fetches PR via `gh` CLI into temp dir (diff, changed files, description, commits, HEAD SHA, repo info) |
| **Lens selection** | 6 lenses with auto-detect + user focus arguments; waits for confirmation |
| **Agent spawning** | Explicit JSON output instructions in spawn prompts |
| **Malformed output** | 4-step extraction strategy (find JSON fence → extract → parse → fallback to general finding) |
| **Synthesis** | 9-step pipeline: parse → aggregate → validate line numbers → deduplicate → prioritise/cap → verdict → cross-cutting themes → compose summary → compose comment bodies |
| **Deduplication** | Proximity + semantic similarity required; merge rules defined |
| **Presentation** | Two-part preview: summary body + inline comments grouped by file |
| **Actions** | 5 options: post review, change verdict, edit comments, discuss, re-run lenses |
| **Posting** | Single GitHub Reviews API call with summary body + up to 10 inline comments |
| **Error handling** | Specific handlers for `gh` failures, 422 errors, stale SHA |
| **Guidelines** | 10 numbered guidelines |
| **What NOT to Do** | 9 items |

#### review-plan Command (older)

| Feature | Implementation |
|---------|---------------|
| **Input handling** | Reads plan file + referenced files directly |
| **Lens selection** | 5 lenses with auto-detect + user focus arguments; waits for confirmation |
| **Agent spawning** | Simple prompts: "Read it fully, explore the codebase, return structured analysis" |
| **Malformed output** | No handling specified |
| **Synthesis** | 5-step pipeline: collect by severity → cross-cutting themes → tradeoffs → deduplicate → prioritise |
| **Deduplication** | Simple: "where multiple lenses flag the same underlying issue, consolidate" |
| **Presentation** | Single narrative format with sections: Overall Assessment, Cross-Cutting Themes, Findings by Severity, Tradeoff Analysis, Strengths, Recommended Changes |
| **Actions** | Collaborative iteration: discuss → edit plan → summarise changes → offer re-review |
| **Posting** | N/A — edits the plan file directly using the Edit tool |
| **Error handling** | None specified |
| **Guidelines** | 8 numbered guidelines |
| **What NOT to Do** | 6 items |

### 3. Lens Coverage Differences

| Lens | PR Command | Plan Command | Notes |
|------|-----------|-------------|-------|
| Architecture | ✅ `pr-architecture-reviewer` | ✅ `plan-architecture-reviewer` | Similar scope |
| Security | ✅ `pr-security-reviewer` | ✅ `plan-security-reviewer` | PR: OWASP-focused on code; Plan: STRIDE + OWASP on design |
| Code Quality | ✅ `pr-code-quality-reviewer` | ✅ `plan-code-quality-reviewer` | Similar scope |
| Test Coverage | ✅ `pr-test-coverage-reviewer` | ❌ Not present | PR-specific lens |
| Standards | ✅ `pr-standards-reviewer` | ✅ `plan-standards-reviewer` | Plan has WCAG/accessibility that PR lacks |
| Usability | ✅ `pr-usability-reviewer` | ✅ `plan-usability-reviewer` | Similar scope |

Test Coverage makes sense as PR-only (reviews actual test code changes).
The plan command reviewing a design document wouldn't have test code to evaluate.

### 4. Agent Structure Comparison (Matched Pair: Architecture)

**Shared structure** (both agents):
- Frontmatter (name, description, tools)
- Intro paragraph
- Core Responsibilities (3 groups)
- Analysis Strategy (multi-step)
- Output Format
- Important Guidelines
- What NOT to Do
- Closing "Remember:" paragraph

**PR agent additions** (not in plan agent):
- "Anchor Findings to Diff Locations" analysis step
- JSON Output Format with Field Reference subsection
- Multi-Line Comment API Mapping subsection
- Severity Emoji Prefixes subsection
- Comment Body Format subsection with example
- 4 additional guidelines (anchor to diff, self-contained bodies, emoji
  prefixes, JSON-only output)
- 1 additional What NOT to Do item (no out-of-diff lines)

**PR agent differences in Core Responsibilities**:
- PR: "Evaluate Structural **Impact of Changes**" (reviews actual diffs)
- Plan: "Evaluate Structural **Integrity**" (reviews proposed design)
- PR: "Assess Architectural **Consistency and Drift**"
- Plan: "Assess Evolutionary **Fitness and Tradeoffs**"

**PR agent differences in Analysis Strategy**:
- PR Step 1: Reads diff files from temp directory
- Plan Step 1: Reads plan file directly
- PR has "Beyond-the-Diff Impact" step
- Plan has "Identify Tradeoffs and Sensitivity Points" step

### 5. Agent Content Quality Differences

The PR agents' Core Responsibilities and Analysis Strategy sections have been
refined from the plan agents. For example:

**Security lens**:
- Plan agent: 3 Core Responsibilities → 5 Analysis Steps (STRIDE-based)
- PR agent: 3 Core Responsibilities → 5 Analysis Steps (OWASP-based on code) +
  1 anchoring step

The PR agents are more focused on **concrete code** while plan agents focus on
**proposed design**. This is a natural difference, not a deficiency.

**Standards lens**:
- Plan agent has WCAG/accessibility assessment (not in PR agent)
- PR agent has `"LEFT"`/`"RIGHT"` side awareness (not relevant for plans)

### 6. Post-Review Workflow

This is the most fundamentally different area because the two commands have
different *destinations* for their output:

- **review-pr**: Posts to GitHub (Reviews API with inline comments)
- **review-plan**: Edits the plan file directly, then offers re-review

The plan command's collaborative iteration workflow (Steps 6-7) has no equivalent
in the PR command, and shouldn't — it's appropriate for plan iteration. Similarly,
the PR command's GitHub API posting (Step 6) has no equivalent in the plan command.

## Architecture Insights

### What to Align (Recommended Changes)

1. **Plan agent output format → JSON**: Adopt the same structured JSON output
   format for plan agents, adapted for plan context (no `path`/`line`/`side`/
   `end_line` since there are no diff hunks — use `location` field instead):

   ```json
   {
     "lens": "architecture",
     "summary": "2-3 sentence assessment",
     "strengths": ["..."],
     "findings": [
       {
         "severity": "critical",
         "confidence": "high",
         "lens": "architecture",
         "location": "Phase 2, Section 3: Database Migration",
         "title": "Finding title",
         "body": "🔴 **Architecture**\n\nIssue...\n\n**Impact**: ...\n\n**Suggestion**: ..."
        }
     ]
   }
   ```

   Note: For plans, there's no meaningful distinction between "anchored" and
   "general" findings (no diff hunks). All findings reference a plan section.
   A single `findings` array suffices instead of `comments` + `general_findings`.

2. **Plan command synthesis → structured pipeline**: Adopt the PR command's
   structured aggregation pattern:
   - Parse JSON outputs
   - Aggregate across agents
   - Deduplicate with semantic similarity requirement
   - Prioritise by severity × confidence
   - Identify cross-cutting themes and tradeoffs
   - Compose structured summary with emoji prefixes

3. **Malformed output handling**: Add the same 4-step JSON extraction strategy
   and fallback to the plan command.

4. **Severity emoji prefixes**: Adopt the same 🔴/🟡/🔵 system in plan agent
   findings.

5. **Self-contained finding bodies**: Plan agent findings should follow the
   same `emoji + **Lens** + issue + impact + suggestion` body format.

6. **`lens` field on every finding**: Enables attribution after merging,
   consistent with PR agents.

7. **Plan command guidelines**: Align numbering and add equivalent guidelines
   where applicable (particularly around structured output parsing and emoji
   prefixes).

### What NOT to Align (Intentional Differences to Preserve)

1. **No `path`/`line`/`side`/`end_line` for plan agents** — plans don't have
   diff hunks. Use `location` (plan section reference) instead.

2. **No `comments` vs `general_findings` split** — plans don't need this
   distinction. A single `findings` array is cleaner.

3. **No "Anchor Findings to Diff Locations" step** — plan agents reference
   plan sections, not diff lines. The equivalent is already embedded in their
   existing analysis steps.

4. **No GitHub API posting** — plan reviews are consumed in-conversation and
   applied via plan editing. The collaborative iteration workflow (Steps 6-7)
   should be preserved.

5. **No Test Coverage lens** — not applicable to plan review.

6. **No Multi-Line Comment API Mapping** — PR-specific concern.

7. **Plan-specific Core Responsibilities and Analysis Steps** — the plan agents
   rightly focus on proposed design rather than actual code. Their analysis
   steps should stay plan-focused.

8. **Collaborative iteration (Steps 6-7)** — unique to plan review and
   valuable. Should be preserved and potentially enhanced.

## Code References

### PR Command and Agents
- `commands/review-pr.md` — 479 lines, recently updated with inline comments
- `agents/pr-architecture-reviewer.md` — 269 lines (template for all 6)
- `agents/pr-security-reviewer.md` — 271 lines
- `agents/pr-code-quality-reviewer.md` — 267 lines
- `agents/pr-test-coverage-reviewer.md` — 279 lines
- `agents/pr-standards-reviewer.md` — 275 lines
- `agents/pr-usability-reviewer.md` — 273 lines

### Plan Command and Agents
- `commands/review-plan.md` — 323 lines
- `agents/plan-architecture-reviewer.md` — 145 lines
- `agents/plan-code-quality-reviewer.md` — 174 lines
- `agents/plan-security-reviewer.md` — 157 lines
- `agents/plan-standards-reviewer.md` — 195 lines
- `agents/plan-usability-reviewer.md` — 192 lines

### Related Documents
- `meta/research/2026-02-22-pr-review-agents-design.md` — original PR agent design research
- `meta/research/2026-02-22-pr-review-inline-comments.md` — inline comments research
- `meta/plans/2026-02-22-pr-review-agents.md` — PR agent implementation plan
- `meta/plans/2026-02-22-pr-review-inline-comments.md` — inline comments plan

## Summary of Upgrade Scope

### Changes per Plan Agent (5 files)

Each plan agent needs:
1. Replace Output Format section with JSON schema (adapted for plan context)
2. Add severity emoji prefixes and body format documentation
3. Add guidelines about JSON output and self-contained bodies
4. Add What NOT to Do item about JSON-only output
5. Keep existing Core Responsibilities and Analysis Steps unchanged

### Changes to Plan Command (1 file)

The `review-plan` command needs:
1. Add malformed output handling (JSON extraction + fallback)
2. Replace synthesis pipeline with structured aggregation
3. Update presentation format to use emoji prefixes and structured layout
4. Update spawn prompts to request JSON output
5. Preserve collaborative iteration workflow (Steps 6-7) — possibly enhance
6. Add relevant guidelines from PR command (where applicable)

### Estimated Scope

- 5 agent files × ~4 edits each = ~20 agent edits
- 1 command file × ~6 edits = ~6 command edits
- Similar in scope to the PR inline comments implementation

## Open Questions

1. Should plan agent findings include a `location` field referencing a plan
   section, or is the location information sufficient within the `body` text?
   (Recommendation: explicit `location` field for structured deduplication.)

2. Should the plan command present a structured preview similar to the PR
   command's two-part preview, or is the current narrative format more
   appropriate for plan review? (Recommendation: adopt structured preview but
   adapt grouping — by severity rather than by file.)

3. Should the plan command support a re-review that diffs against the previous
   review's findings (tracking resolved/new issues)? It already has a basic
   re-review in Step 7, but it could be enhanced with the structured JSON
   format. (Recommendation: enhance after initial alignment — track by finding
   `title` + `lens` to detect resolved vs new.)
