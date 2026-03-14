---
name: plan-review-output-format
description: Output format specification for plan review agents. Defines the
  JSON schema, field reference, severity emoji prefixes, and finding body format
  for plan reviews.
user-invocable: false
disable-model-invocation: true
---

# Plan Review Output Format

## JSON Schema

Return your analysis as a JSON code block. Do not include any text before or
after the JSON block — the orchestrator will parse this output directly.

```json
{
  "lens": "<lens-identifier>",
  "summary": "2-3 sentence assessment from this lens perspective.",
  "strengths": [
    "Positive observation about what the plan gets right from this lens perspective"
  ],
  "findings": [
    {
      "severity": "critical",
      "confidence": "high",
      "lens": "<lens-identifier>",
      "location": "Phase 2, Section 3: Database Migration",
      "title": "Brief finding title",
      "body": "🔴 **<Lens Name>**\n\n[Issue description — 1-2 sentences with enough context to understand standalone].\n\n**Impact**: [Why this matters — 1 sentence].\n\n**Suggestion**: [Concrete fix — 1-2 sentences]."
    }
  ]
}
```

## Field Reference

- **lens**: Agent lens identifier (e.g., `"architecture"`, `"security"`,
  `"test-coverage"`, `"code-quality"`, `"standards"`, `"usability"`,
  `"performance"`)
- **summary**: 2-3 sentence assessment from this lens perspective. Reflect the
  key dimensions from the lens's Core Responsibilities. This is where holistic
  assessment lives, beyond individual findings.
- **strengths**: Positive observations (fed into the review summary — never
  posted as individual findings)
- **findings**: All findings, each referencing a location in the plan
  - **severity**: One of `"critical"`, `"major"`, `"minor"`, `"suggestion"`
  - **confidence**: One of `"high"`, `"medium"`, `"low"`
  - **lens**: The lens identifier (same value as the top-level `lens` field).
    Included on each finding so the orchestrator can attribute findings after
    merging outputs from multiple agents.
  - **location**: Human-readable reference to the plan section where the
    finding is most relevant (e.g., "Phase 2: API Endpoints",
    "Implementation Approach", "Phase 1: Data Model")
  - **title**: Brief title for the finding (used in the summary index)
  - **body**: Self-contained finding body. See "Finding Body Format" below.

## Severity Emoji Prefixes

Use these at the start of each finding `body`:
- `🔴` for `"critical"` severity
- `🟡` for `"major"` severity
- `🔵` for `"minor"` and `"suggestion"` severity

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
🔴 **Architecture**

The plan proposes a direct dependency from the API layer to the database schema
with no service abstraction. This couples the presentation layer to the data
model.

**Impact**: Database schema changes will ripple into the API layer, breaking
the dependency rule.

**Suggestion**: Introduce a service layer to mediate between the API handlers
and the data access layer.
```

**Output only the JSON block** — do not include additional prose, narrative
analysis, or markdown outside the JSON code fence. The orchestrator parses
your output as JSON.
