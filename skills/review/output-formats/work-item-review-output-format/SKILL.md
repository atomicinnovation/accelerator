---
name: work-item-review-output-format
description: Output format specification for work-item review agents. Defines the
  JSON schema, field reference, severity emoji prefixes, and finding body format
  for work-item reviews. Used by review orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Work-Item Review Output Format

## JSON Schema

Return your analysis as a JSON code block. Do not include any text before or
after the JSON block — the orchestrator will parse this output directly.

```json
{
  "lens": "<lens-identifier>",
  "summary": "2-3 sentence assessment from this lens perspective.",
  "strengths": [
    "Positive observation about what the work item gets right from this lens perspective"
  ],
  "findings": [
    {
      "severity": "critical",
      "confidence": "high",
      "lens": "<lens-identifier>",
      "location": "Acceptance Criteria",
      "title": "Brief finding title",
      "body": "🔴 **<Lens Name>**\n\n[Issue description — 1-2 sentences with enough context to understand standalone].\n\n**Impact**: [Why this matters — 1 sentence].\n\n**Suggestion**: [Concrete fix — 1-2 sentences]."
    }
  ]
}
```

An optional `synthetic` boolean field may appear on findings. It is set to
`true` only by the orchestrator's malformed-agent fallback — agents must never
emit it.

## Field Reference

- **lens**: Agent lens identifier (e.g., `"completeness"`, `"scope"`, …)
- **summary**: 2-3 sentence assessment from this lens perspective. Reflect the
  key dimensions from the lens's Core Responsibilities. This is where holistic
  assessment lives, beyond individual findings.
- **strengths**: Positive observations (fed into the review summary — never
  posted as individual findings)
- **findings**: All findings, each referencing a location in the work item
  - **severity**: One of `"critical"`, `"major"`, `"minor"`, `"suggestion"`
  - **confidence**: One of `"high"`, `"medium"`, `"low"`
  - **lens**: The lens identifier (same value as the top-level `lens` field).
    Included on each finding so the orchestrator can attribute findings after
    merging outputs from multiple agents.
  - **location**: Human-readable reference to the work item section where the
    finding is most relevant (e.g., `"Summary"`, `"Acceptance Criteria"`,
    `"Dependencies"`, `"Open Questions"`, `"Context"`, `"Requirements"`,
    `"Frontmatter: type"`)
  - **title**: Brief title for the finding (used in the summary index)
  - **body**: Self-contained finding body. See "Finding Body Format" below.

The canonical source of valid lens identifiers is the Lens Catalogue emitted
by `config-read-review.sh work-item`. See the Lens Catalogue for the current
list.

## Severity Emoji Prefixes

Use these **actual Unicode emoji characters** at the start of each finding `body`:
- `🔴` for `"critical"` severity
- `🟡` for `"major"` severity
- `🔵` for `"minor"` and `"suggestion"` severity

**IMPORTANT**: Use the actual Unicode emoji characters shown above (🔴 🟡 🔵), NOT
text shortcodes like `:red_circle:`, `:yellow_circle:`, or `:blue_circle:`. The
output is rendered as markdown, not Slack/Discord, so shortcodes will appear as
literal text.

## Finding Body Format

Each finding `body` should follow this structure:

```
[emoji] **[Lens Name]**

[Issue description — 1-2 sentences, standalone context].

**Impact**: [Why this matters].

**Suggestion**: [Concrete fix].
```

Example:

```
🔴 **Completeness**

The work item has no Acceptance Criteria section, so there is no definition of
what "done" means.

**Impact**: Implementers cannot verify when the work is complete, risking
scope drift or premature closure.

**Suggestion**: Add an Acceptance Criteria section with at least two specific,
testable bullets.
```

**Output only the JSON block** — do not include additional prose, narrative
analysis, or markdown outside the JSON code fence. The orchestrator parses
your output as JSON.
