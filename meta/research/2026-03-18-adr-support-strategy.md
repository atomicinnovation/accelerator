---
date: "2026-03-18T02:43:24+00:00"
researcher: Toby Clemson
git_commit: 2bfac71efe3c5cd83ea1fa3b48b69fa805c4919f
branch: add-adrs
repository: accelerator
topic: "ADR support strategy - adding architecture decision record skills to the accelerator plugin"
tags: [ research, adr, architecture-decisions, skills, meta-directory ]
status: complete
last_updated: "2026-03-18"
last_updated_by: Toby Clemson
---

# Research: ADR Support Strategy

**Date**: 2026-03-18T02:43:24+00:00
**Researcher**: Toby Clemson
**Git Commit**: 2bfac71efe3c5cd83ea1fa3b48b69fa805c4919f
**Branch**: add-adrs
**Repository**: accelerator

## Research Question

How should we add architecture decision record (ADR) management skills to the
accelerator plugin? ADRs should be stored in `meta/`, support extraction from
existing meta documents (research, plans), offer interactive generation, and
enforce an append-only timeseries where ADRs become immutable once they move
beyond `proposed` status.

## Summary

The accelerator plugin's existing architecture — filesystem-mediated skill
chaining, YAML frontmatter conventions, date-prefixed naming, and the
`meta/` directory as persistent shared memory — maps naturally onto ADR
management. The `documents-locator` agent already references a `meta/decisions/`
directory (currently unused), making it the obvious home for ADRs. We propose
three new skills (`create-adr`, `extract-adrs`, `review-adr`) under a new
`skills/decisions/` category, with an append-only enforcement model that
leverages ADR status to determine mutability — `proposed` ADRs are freely
editable, while any other status renders the ADR immutable except for
permitted status transitions.

## Detailed Findings

### 1. Existing Codebase Patterns Relevant to ADRs

#### Skill Structure

Every skill in the plugin follows the same pattern:

- A `SKILL.md` file with YAML frontmatter (`name`, `description`,
  `argument-hint`, `disable-model-invocation`)
- Placed at `skills/<category>/<skill-name>/SKILL.md`
- Registered in `.claude-plugin/plugin.json` by adding the parent directory path
- Optional companion scripts in a `scripts/` subdirectory

Skills fall into categories: `github/`, `planning/`, `research/`, `review/`,
`vcs/`. A new `decisions/` category would follow this convention.

#### Meta Directory Conventions

The `meta/` directory serves as persistent shared memory with these conventions:

- **Naming**: `YYYY-MM-DD-[ENG-XXXX-]description.md` (date-prefixed,
  kebab-case)
- **Frontmatter**: Research documents use rich YAML frontmatter; plans use none
- **Cross-references**: Backtick-quoted `meta/` relative paths in
  `## References` sections
- **Ownership**: Each subdirectory is "owned" by a specific skill
- **Discovery**: The `documents-locator` agent already lists `meta/decisions/`
  in its search structure (line 50), though no files exist there yet

#### Skill Chaining

Skills chain through filesystem artifacts, not direct invocation. The existing
lifecycle is:

```
research-codebase → create-plan → review-plan → implement-plan → validate-plan
                                                                       ↓
                                               describe-pr → review-pr → commit
```

ADR skills would extend this by:

- Consuming outputs from `research-codebase` and `create-plan`
- Producing decision records that inform future planning and implementation

#### Agent Ecosystem

The plugin has seven reusable agents. ADR skills can leverage:

- `documents-locator` — discover existing meta documents to extract from
- `documents-analyser` — deep-read documents to identify decisions
- `codebase-locator` / `codebase-analyser` — understand code context for
  decisions

### 2. ADR Best Practices from the Ecosystem

#### Template Formats

The most widely used ADR formats are:

**Michael Nygard's Original (2011)** — Title, Status, Context, Decision,
Consequences. One to two pages maximum. Numbers never reused. Stored in version
control.

**MADR (Markdown Architectural Decision Records)** — Extends Nygard with
Decision Drivers, Considered Options, Pros/Cons analysis, and Confirmation
sections. Offers four variants (full, minimal, bare, bare-minimal) to scale
from quick captures to detailed records.

**Y-Statements** — Single-sentence format: "In the context of [X], facing [Y],
we decided for [Z] to achieve [Q], accepting [D]." Good for quick capture that
can be expanded later.

**Recommendation**: Use a hybrid inspired by Nygard and MADR — keep it concise
(Nygard's spirit) but include Considered Options and Decision Drivers (MADR's
structure). This aligns with the plugin's philosophy of structured,
machine-parseable documents.

#### Lifecycle and Statuses

Standard ADR lifecycle:

```
Proposed → Accepted → [immutable]
                   ↘ Superseded (by ADR-NNN)
                   ↘ Deprecated (with reason)
Proposed → Rejected (with reason)
```

Key principle: **Once accepted, an ADR becomes immutable.** Only the status
field may change (to "Superseded by ADR-NNN" or "Deprecated"). New insights
require a new ADR, not modification of the old one.

#### Append-Only Enforcement

From AWS and Microsoft guidance:

- "When the team accepts an ADR, it becomes immutable"
- "The ADR serves as an append-only log"
- Supersession creates a new ADR that references the old one
- Old ADR content remains untouched; only status changes
- Numbers are never reused

This aligns with the requirement that ADRs become immutable once they leave
`proposed` status.

#### ADR Extraction Patterns

Academic research (arxiv 2601.19548, January 2025) describes automated
extraction from textual artifacts using NLP/LLMs with 84-91% accuracy. The
practical "kernel of truth" method (Equal Experts) involves:

1. Discover one-line decision statements from documents
2. Reconstruct context (when, why)
3. Expand into full ADRs via LLM
4. Critique/review before human sign-off

#### What Makes a Good ADR

Key qualities (Olaf Zimmermann):

- Functions as executive summary, verdict, letter of intent, and action plan
- Balanced pros and cons with tradeoffs explained
- Assertive writing with active voice ("We will...")
- Length matched to problem complexity

Key anti-patterns to avoid:

- **Fairy Tale**: Only pros, no cons
- **Dummy Alternative**: Non-viable options to make preferred choice look good
- **Mega-ADR**: Multi-page documents crammed with detail
- **Blueprint in Disguise**: Reads like a cookbook, not a decision journal

### 3. Proposed ADR Template

```markdown
---
adr_id: ADR-NNNN
date: "YYYY-MM-DDTHH:MM:SS+00:00"
author: Author Name
status: proposed | accepted | rejected | superseded | deprecated
superseded_by: ADR-NNNN  # only if status is superseded
supersedes: ADR-NNNN     # only if this ADR replaces another
deprecated_reason: ""     # only if status is deprecated
tags: [tag1, tag2]
---

# ADR-NNNN: Title as Short Noun Phrase

**Date**: YYYY-MM-DD
**Status**: Proposed
**Author**: Author Name

## Context

[Forces at play — technological, political, social, project-specific.
Value-neutral language describing facts, not advocating.]

## Decision Drivers

- [Driver 1]
- [Driver 2]

## Considered Options

1. **Option A** — Brief description
2. **Option B** — Brief description
3. **Option C** — Brief description

## Decision

[The chosen option and why, stated in active voice: "We will..."]

## Consequences

### Positive

- [Consequence 1]

### Negative

- [Consequence 1]

### Neutral

- [Consequence 1]

## References

- `meta/research/YYYY-MM-DD-topic.md` — Related research
- `meta/plans/YYYY-MM-DD-topic.md` — Related plan
- `meta/decisions/ADR-NNNN.md` — Related/superseded ADR
```

### 4. Proposed Skills

#### Skill 1: `create-adr` (Interactive ADR Generation)

**Path**: `skills/decisions/create-adr/SKILL.md`
**Invocation**: `/accelerator:create-adr [topic or description]`

**Workflow**:

1. Accept a topic/description (or prompt interactively)
2. Determine the next ADR number by scanning `meta/decisions/` for the highest
   existing `ADR-NNNN` number
3. Spawn agents to gather context:
  - `documents-locator` to find related research, plans, and existing ADRs
  - `codebase-locator` to find relevant code
4. Present gathered context and ask clarifying questions
5. Draft the ADR using the template above
6. Present to user for review and iteration
7. Write to `meta/decisions/ADR-NNNN-description.md`

**Key design decisions**:

- Uses sequential `ADR-NNNN` numbering (not date-prefixed) because ADRs are a
  timeseries where ordering matters and numbers must never be reused
- File naming is `ADR-NNNN-description.md` (e.g., `ADR-0001-use-jujutsu.md`)
  to maintain sort order and human readability
- New ADRs start with status `proposed`
- The skill should guide the user through context, options, and consequences
  interactively

#### Skill 2: `extract-adrs` (Extract ADRs from Meta Documents)

**Path**: `skills/decisions/extract-adrs/SKILL.md`
**Invocation**: `/accelerator:extract-adrs [@meta/research/doc.md ...]`

**Workflow**:

1. Accept one or more meta document paths (or scan all of `meta/` if none
   specified)
2. Read the specified documents fully
3. Spawn `documents-analyser` agents to identify architectural decisions within
   each document — look for:
  - Explicit decision statements ("We decided...", "We will use...")
  - Option comparisons and tradeoffs
  - Technology selections
  - Pattern/approach choices
  - Constraint acknowledgements
4. Present discovered decisions as a numbered list with Y-statement summaries
5. Let the user select which decisions to capture as ADRs
6. For each selected decision, generate a full ADR using `create-adr` template
   with context pre-filled from the source document
7. Write each ADR to `meta/decisions/` with sequential numbering
8. Add cross-references back to source documents

**Key design decisions**:

- Extracts the "kernel of truth" from existing documents
- User selects which decisions are worth recording (not everything is
  architecturally significant)
- Source document references are preserved bidirectionally
- Batch creation with sequential numbering

#### Skill 3: `review-adr` (Review and Accept/Reject ADRs)

**Path**: `skills/decisions/review-adr/SKILL.md`
**Invocation**: `/accelerator:review-adr [@meta/decisions/ADR-NNNN.md]`

**Workflow**:

1. Accept an ADR path (or list proposed ADRs for selection)
2. Read the ADR fully
3. Check the ADR is in `proposed` status (reject if already accepted/etc.)
4. Review for quality using criteria from Zimmermann's guidance:
  - Context completeness
  - Balanced options analysis (no Fairy Tale or Dummy Alternative)
  - Clear decision statement
  - Honest consequences (positive, negative, neutral)
  - Appropriate length
5. Present review findings and suggestions
6. Offer actions:
  - **Accept**: Change status to `accepted`, add acceptance date
  - **Reject**: Change status to `rejected`, add rejection reason
  - **Revise**: Suggest specific improvements (only if still `proposed`)
7. Transition the ADR status (proposed ADRs become immutable once accepted
   or rejected)

**Key design decisions**:

- Quality gate before acceptance
- Only `proposed` ADRs can be modified
- Acceptance is a deliberate action, not automatic

### 5. Append-Only Enforcement Strategy

The append-only requirement needs careful design. The approach:

#### Mutability Rules

| ADR Status   | Content editable? | Permitted transitions                    |
|--------------|-------------------|------------------------------------------|
| `proposed`   | Yes               | → `accepted`, → `rejected`               |
| `accepted`   | No                | → `superseded` (by new ADR), → `deprecated` |
| `rejected`   | No                | None (terminal)                          |
| `superseded` | No                | None (terminal)                          |
| `deprecated` | No                | None (terminal)                          |

The key principle: **`proposed` is the only status where content can be freely
edited.** Once an ADR transitions to any other status, only further status
transitions are permitted — never content changes. This is simple, predictable,
and does not require checking VCS state.

#### Enforcement Mechanism

Rather than a hook (which would add complexity), enforcement should be built
into the skills themselves:

1. **`create-adr`**: Always creates new files, never modifies existing ones.
   New ADRs start with status `proposed`.
2. **`review-adr`**: Reads the ADR's `status` field from frontmatter before
   allowing any changes:
   - If `proposed`: content and status changes are both permitted
   - If any other status: only permitted status transitions are allowed (e.g.,
     `accepted` → `superseded`), never content changes
3. **Supersession**: When a decision needs to change, `create-adr` is used with
   a `--supersedes ADR-NNNN` argument, which:
   - Creates a new ADR with `supersedes: ADR-NNNN` in frontmatter
   - Updates the old ADR's status to `superseded` and adds
     `superseded_by: ADR-MMMM` (this is the only permitted modification to
     a non-proposed ADR)
4. **Deprecation**: Similar to supersession — a skill action updates only the
   status field and adds `deprecated_reason`

#### Why Not a Hook?

A pre-tool-use hook could enforce immutability, but:

- It would need to parse frontmatter on every file write (expensive and fragile)
- The existing `vcs-guard.sh` hook shows the pattern works for command blocking,
  but content-level protection is a different concern
- Skill-level enforcement is simpler, more transparent, and sufficient since
  ADRs are only modified through these skills
- Status is a property of the document itself, making it self-describing —
  any skill can check it without external state

### 6. Integration Points

#### Plugin Registration

Add to `.claude-plugin/plugin.json`:

```json
"skills": [
"./skills/vcs/",
"./skills/github/",
"./skills/planning/",
"./skills/research/",
"./skills/decisions/",
"./skills/review/lenses/",
"./skills/review/output-formats/"
]
```

#### Documents Locator Agent

The `documents-locator` agent already references `meta/decisions/` (line 50) —
no changes needed to the agent itself.

#### README Update

Add `decisions/` to the meta directory table:

| Directory    | Purpose                              | Written by                   |
|--------------|--------------------------------------|------------------------------|
| `decisions/` | Architecture decision records (ADRs) | `create-adr`, `extract-adrs` |

#### Development Loop Extension

The ADR skills extend the development loop:

```
research-codebase → create-plan → implement-plan
       ↓                ↓               ↓
  meta/research/    meta/plans/    checked-off plan
       ↓                ↓
  extract-adrs ←────────┘
       ↓
  meta/decisions/
       ↓
  review-adr → accepted ADRs inform future research & planning
```

ADRs sit alongside the existing research → plan → implement cycle, capturing
the decisions that emerge from research and planning.

#### Cross-Referencing

ADRs should cross-reference their source documents:

- `extract-adrs` automatically adds `## References` linking to source
  research/plan documents
- `create-adr` prompts for related documents
- Future `create-plan` and `research-codebase` invocations could discover
  relevant ADRs via `documents-locator`

### 7. ADR Numbering and Naming

**Format**: `ADR-NNNN-description.md`

**Examples**:

- `ADR-0001-use-jujutsu-for-version-control.md`
- `ADR-0002-filesystem-mediated-skill-chaining.md`
- `ADR-0003-append-only-adr-lifecycle.md`

**Rationale for sequential numbering over date-prefixed**:

- ADRs form a strict timeseries; sequential numbers make ordering unambiguous
- Numbers serve as stable identifiers for cross-references (`superseded_by:
  ADR-0005`)
- Date is captured in frontmatter, so no information is lost
- This follows the convention established by Nygard's original ADR format and
  adopted by adr-tools, MADR, and most ADR tooling

**Number assignment**: `create-adr` and `extract-adrs` scan `meta/decisions/`
for the highest existing number and increment. A helper script
(`scripts/adr-next-number.sh`) could handle this to keep it DRY across skills.

### 8. Companion Scripts

Following the pattern of `research-metadata.sh`, ADR skills could use:

**`scripts/adr-next-number.sh`**: Scans `meta/decisions/` for the highest
ADR number and outputs the next one. Used by both `create-adr` and
`extract-adrs`.

**`scripts/adr-read-status.sh`**: Reads the `status` field from an ADR file's
YAML frontmatter and outputs it. Used by skills to determine mutability without
parsing frontmatter themselves. Returns a non-zero exit code if the file has no
valid frontmatter.

## Code References

- `.claude-plugin/plugin.json:9-16` — Skill registration array
- `agents/documents-locator.md:17-18` — Already references `meta/decisions/`
- `agents/documents-locator.md:44-53` — Meta directory structure listing
- `skills/research/research-codebase/SKILL.md:97-104` — Naming convention
  pattern
- `skills/research/research-codebase/scripts/research-metadata.sh` — Companion
  script pattern
- `skills/planning/create-plan/SKILL.md:197-205` — Plan file creation pattern
- `scripts/vcs-common.sh` — VCS abstraction pattern for helper scripts
- `README.md:66-79` — Meta directory documentation

## Architecture Insights

1. **Convention over configuration**: The plugin relies on directory structure
   and naming conventions rather than explicit configuration. ADR skills should
   follow this — `meta/decisions/` is discovered by convention, not configured.

2. **Skill-level enforcement over hooks**: While hooks exist for VCS concerns
   (guarding git commands in jj repos), file-level immutability is better
   enforced within the skills themselves, keeping the hook system focused on
   cross-cutting VCS concerns.

3. **Agent delegation for context gathering**: All complex skills delegate
   research to sub-agents (`documents-locator`, `codebase-analyser`). ADR
   skills should follow this pattern rather than doing deep file reads in the
   main context.

4. **Filesystem as interface**: Skills communicate through `meta/` artifacts.
   ADRs become another artifact type that feeds back into the research → plan →
   implement loop.

5. **Progressive structure**: The plugin offers different levels of formality
   (Y-statements for quick capture via `extract-adrs`, full ADRs via
   `create-adr`). This matches MADR's approach of scaling from minimal to
   detailed.

## Related Research

- [Documenting Architecture Decisions — Michael Nygard](https://www.cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
- [MADR — Markdown Architectural Decision Records](https://adr.github.io/madr/)
- [ADR GitHub Organisation — Templates and Tooling](https://adr.github.io/)
- [AWS Prescriptive Guidance — ADR Process](https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/adr-process.html)
- [Microsoft Azure Well-Architected Framework — ADR](https://learn.microsoft.com/en-us/azure/well-architected/architect-role/architecture-decision-record)
- [How to Create ADRs and How Not To — Olaf Zimmermann](https://ozimmer.ch/practices/2023/04/03/ADRCreation.html)
- [adr-tools — Nat Pryce](https://github.com/npryce/adr-tools)
- [Log4brains](https://github.com/thomvaill/log4brains)
- [Accelerating ADRs with Generative AI — Equal Experts](https://www.equalexperts.com/blog/our-thinking/accelerating-architectural-decision-records-adrs-with-generative-ai/)
- [From Scattered to Structured — arxiv 2601.19548](https://arxiv.org/html/2601.19548)
- [joelparkerhenderson/architecture-decision-record](https://github.com/joelparkerhenderson/architecture-decision-record)

## Open Questions

1. **Should `extract-adrs` also scan code comments?** The initial scope targets
   meta documents only, but architectural decisions sometimes live in code
   comments (e.g., `// We chose X over Y because...`). This could be a future
   enhancement.

2. **ADR index/table of contents**: Should we generate an `index.md` in
   `meta/decisions/` listing all ADRs with their status? adr-tools does this.
   It would aid discovery but adds a maintenance burden (another file to keep
   in sync).

3. **Should `review-adr` use the existing review lens/agent pattern?** A
   dedicated "adr-quality-lens" could be created for the `reviewer` agent,
   providing consistent quality assessment. This would be elegant but may be
   over-engineering for the initial version.

4. **Batch extraction UX**: When `extract-adrs` finds many decisions in a
   document, should it present them all at once for selection, or walk through
   them one by one? Batch is faster; one-by-one allows more careful
   consideration.

5. **ADR templates**: Should we support multiple ADR templates (full, minimal)
   like MADR does? Or start with a single template and add variants later?
