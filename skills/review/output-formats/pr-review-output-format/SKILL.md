---
name: pr-review-output-format
description: Output format specification for PR review agents. Defines the JSON
  schema, field reference, severity emoji prefixes, and comment body format for
  PR reviews.
user-invocable: false
disable-model-invocation: true
---

# PR Review Output Format

## JSON Schema

Return your analysis as a JSON code block. Do not include any text before or
after the JSON block — the orchestrator will parse this output directly.

```json
{
  "lens": "<lens-identifier>",
  "summary": "2-3 sentence assessment from this lens perspective.",
  "strengths": [
    "Positive observation about what the PR gets right from this lens perspective"
  ],
  "comments": [
    {
      "path": "src/example.ts",
      "line": 42,
      "end_line": null,
      "side": "RIGHT",
      "severity": "critical",
      "confidence": "high",
      "lens": "<lens-identifier>",
      "title": "Brief finding title",
      "body": "🔴 **<Lens Name>**\n\n[Issue description — 1-2 sentences with enough context to understand standalone].\n\n**Impact**: [Why this matters — 1 sentence].\n\n**Suggestion**: [Concrete fix — 1-2 sentences, optionally with a code snippet]."
    }
  ],
  "general_findings": [
    {
      "severity": "minor",
      "lens": "<lens-identifier>",
      "title": "Cross-cutting finding title",
      "body": "Description of the finding that cannot be anchored to a specific diff line."
    }
  ]
}
```

## Field Reference

- **lens**: Agent lens identifier (e.g., `"architecture"`, `"security"`,
  `"test-coverage"`, `"code-quality"`, `"standards"`, `"usability"`,
  `"performance"`)
- **summary**: 2-3 sentence assessment from this lens perspective
- **strengths**: Positive observations (fed into the review summary — never
  posted as inline comments)
- **comments**: Line-anchored findings for inline PR comments
  - **path**: File path relative to repository root (as shown in the diff
    header, e.g., `src/auth/handler.ts`)
  - **line**: Line number in the file where the comment should appear. For
    `"RIGHT"` side, this is the line number in the new version of the file. For
    `"LEFT"` side, this is the line number in the old version. The line MUST be
    visible in the diff (within a hunk).
  - **end_line**: Last line number for multi-line comments, or `null` for
    single-line. Must be in the same diff hunk as `line`.
  - **side**: `"RIGHT"` for commenting on added, modified, or context lines in
    the new file (the vast majority of comments). `"LEFT"` only for commenting
    on deleted lines.
  - **severity**: One of `"critical"`, `"major"`, `"minor"`, `"suggestion"`
  - **confidence**: One of `"high"`, `"medium"`, `"low"`
  - **lens**: The lens identifier (same value as the top-level `lens` field).
    Included on each comment so the orchestrator can attribute findings after
    merging outputs from multiple agents.
  - **title**: Brief title for the finding (used in the summary index)
  - **body**: Self-contained comment body formatted for a GitHub PR inline
    comment. See "Comment Body Format" below.
- **general_findings**: Findings that cannot be anchored to specific diff lines
  (cross-cutting concerns, missing functionality, architectural observations)
  - **severity**, **lens**, **title**, **body**: Same semantics as in `comments`

## Multi-Line Comment API Mapping

The agent schema uses `line` (start of range) and `end_line` (end of range).
The GitHub API inverts this: `start_line` is the beginning and `line` is the
end. The orchestrator handles this mapping when constructing the API payload.

Example — agent output:
```json
{ "line": 10, "end_line": 15, "side": "RIGHT" }
```
Becomes API payload:
```json
{ "start_line": 10, "start_side": "RIGHT", "line": 15, "side": "RIGHT" }
```

For single-line comments (`end_line` is `null`), the API payload uses only
`line` and `side` — `start_line` and `start_side` are omitted entirely (not
set to `null`).

## Severity Emoji Prefixes

Use these at the start of each comment `body`:
- `🔴` for `"critical"` severity
- `🟡` for `"major"` severity
- `🔵` for `"minor"` and `"suggestion"` severity

## Comment Body Format

Each comment `body` should follow this structure:

```
[emoji] **[Lens Name]**

[Issue description — 1-2 sentences, standalone context].

**Impact**: [Why this matters].

**Suggestion**: [Concrete fix, optionally with a short code snippet].
```

Example:

```
🔴 **Architecture**

This module directly imports from the data layer, bypassing the service
boundary. This couples the API handler to the database schema.

**Impact**: Changes to the database schema will ripple into the API layer.

**Suggestion**: Introduce a service interface to mediate between the API and
data layers.
```

## Diff Anchoring Guidelines

- Every finding in `comments` must reference a line number that is visible in
  the diff (within a hunk). Lines outside diff hunks will cause API errors.
- For findings about added or modified code, use the line number in the new
  file version and set side to `"RIGHT"`.
- For findings about deleted code, use the line number in the old file version
  and set side to `"LEFT"`.
- For findings spanning multiple lines, identify both the start line (`line`)
  and end line (`end_line`) within the same diff hunk.
- For findings that cannot be anchored to a specific diff line, classify them
  as general findings.
- When in doubt, use `general_findings` rather than risking an invalid line
  reference.

**Output only the JSON block** — do not include additional prose, narrative
analysis, or markdown outside the JSON code fence. The orchestrator parses
your output as JSON.
