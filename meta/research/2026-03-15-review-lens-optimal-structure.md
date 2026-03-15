---
date: "2026-03-15T15:46:31+00:00"
researcher: Toby Clemson
git_commit: 54bf3e8f4bf2f289b9ecc6f68c5f4b06859023ad
branch: main
repository: accelerator
topic: "Optimal structure for review lenses — codebase analysis and web research"
tags: [ research, review-lenses, lens-structure, code-review, perspective-based-reading ]
status: complete
last_updated: "2026-03-15"
last_updated_by: Toby Clemson
---

# Research: Optimal Structure for Review Lenses

**Date**: 2026-03-15T15:46:31+00:00
**Researcher**: Toby Clemson
**Git Commit**: 54bf3e8f4bf2f289b9ecc6f68c5f4b06859023ad
**Branch**: main
**Repository**: accelerator

## Research Question

What is the optimal structure for a review lens, informed by analysis of the
existing lenses in this codebase and web research into technical review best
practices, quality frameworks, and AI-assisted review patterns?

## Summary

The existing 7 lenses follow a consistent, well-designed structure that aligns
with industry best practices. Analysis of the codebase pattern combined with web
research into formal review methodologies (Perspective-Based Reading, Fagan
Inspection), quality frameworks (ISO 25010, CISQ, arc42), industry practices
(Google, Microsoft, AWS), and AI-assisted review patterns (specialist-agent
review, severity-driven review) reveals a convergent optimal lens structure. The
key insight from academic research — Perspective-Based Reading — is that each
lens should adopt a focused stakeholder perspective and attempt to produce a
derivative artifact (attack scenarios, failure modes, test cases) rather than
passively scanning. The existing lenses partially embody this principle but
could
be strengthened by making the perspective and generative task more explicit.

## Detailed Findings

### Existing Lens Structure (Codebase Analysis)

All 7 lenses follow an identical structural pattern:

#### YAML Frontmatter

```yaml
---
name: <lens-name>-lens
description: <Lens name> review lens for evaluating <domain>. Used by review
  orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---
```

- `name`: kebab-case identifier with `-lens` suffix
- `description`: one-line description stating the evaluation domain and that it
  is not directly invocable
- `user-invocable: false`: lenses are consumed by orchestrators, not users
- `disable-model-invocation: true`: prevents autonomous model invocation

#### Markdown Body Sections

| Section                           | Purpose          | Pattern                                                                     |
|-----------------------------------|------------------|-----------------------------------------------------------------------------|
| **`# <Lens Name> Lens`**          | Title            | Always `# <Name> Lens`                                                      |
| **`## Core Responsibilities`**    | What to evaluate | 3-4 numbered responsibility groups, each with bulleted sub-items            |
| **Boundary notes**                | Prevent overlap  | Inline notes after responsibilities explaining what belongs to other lenses |
| **`## Key Evaluation Questions`** | How to evaluate  | Bulleted questions grouped by dimension, prefixed with bold category labels |
| **`## Important Guidelines`**     | Review approach  | Pragmatic guidance on how to conduct the review                             |
| **`## What NOT to Do`**           | Scope boundaries | Explicit list of other lenses' concerns to avoid, plus anti-patterns        |
| **Closing "Remember" statement**  | Mission summary  | One-sentence philosophical reminder of the lens's purpose                   |

#### Observed Conventions

1. **Core Responsibilities**: Always 3-4 numbered groups. Each group has a bold
   heading and 4-8 bulleted sub-items. Groups are ordered from most specific to
   most holistic.

2. **Boundary notes**: Appear after responsibilities that border another lens's
   domain. Format: "Note: [What this lens covers] is assessed here. [What the
   other lens covers] is assessed by the [other] lens." These are critical for
   preventing overlap when multiple lenses run in parallel.

3. **Key Evaluation Questions**: Each question uses bold prefix labels
   (e.g., `**Algorithmic complexity**:`, `**Spoofing**:`). Questions are
   phrased as "What/Is/Are/Can" interrogatives. Some lenses (standards,
   security)
   group questions under sub-categories with conditional applicability notes
   (e.g., "when API changes are present").

4. **Important Guidelines**: Always starts with "**Explore the codebase**" and
   includes "**Rate confidence** on each finding". Other guidelines are
   lens-specific but follow a pattern of `**Bold imperative**` followed by an
   em-dash explanation.

5. **What NOT to Do**: Always begins with "Don't review [list of all other
   lenses] — those are other lenses". Followed by lens-specific anti-patterns.
   Some lenses include a specific boundary clarification (e.g., security lens
   clarifies DoS evaluation stays in security vs general performance).

6. **Remember statement**: Always formatted as `Remember: You're evaluating
   whether [core purpose]. [Philosophical guidance].`

### Web Research Findings

#### Perspective-Based Reading (PBR) — Academic Foundation

The strongest theoretical foundation for lens design comes from
Perspective-Based Reading (Basili, Shull et al., University of Maryland):

- **Each reviewer adopts a specific stakeholder perspective** with a structured
  procedure
- **Each perspective produces a derivative artifact** rather than passively
  reading:
  - User perspective → draft use cases
  - Designer perspective → draft architecture
  - Tester perspective → draft test plans
- **Different perspectives find different defect types** — the value is in
  combination
- **Teams using PBR achieve significantly better coverage** than unfocused
  review

This principle translates directly to AI review lenses: each lens should adopt a
perspective (e.g., "review as an attacker", "review as a capacity planner") and
attempt to generate something (attack scenarios, bottleneck analysis, failure
modes).

Sources:

- [How Perspective-Based Reading Can Improve Requirements Inspections (IEEE)](https://ieeexplore.ieee.org/document/869376/)
- [The Empirical Investigation of Perspective-Based Reading (Springer)](https://link.springer.com/article/10.1007/BF00368702)

#### Quality Frameworks — Dimension Coverage

**ISO/IEC 25010 (2023)** defines 9 top-level quality characteristics:

1. Functional Suitability
2. Performance Efficiency
3. Compatibility
4. Usability
5. Reliability
6. Security
7. Maintainability
8. Portability
9. Safety (added in 2023)

**arc42 Quality Model** offers a more practical alternative with 9 adjective-
based dimensions: #reliable, #flexible, #maintainable, #efficient, #usable,
#operable, #suitable, #secure, #safe — containing 216 quality characteristics.

**CISQ** focuses on 4 measurable characteristics: Reliability, Security,
Performance Efficiency, Maintainability — covering 86 critical code quality
rules
mapped to CWE.

The existing 7 lenses map well to these frameworks:

| Existing Lens | ISO 25010                                 | arc42                | CISQ                   |
|---------------|-------------------------------------------|----------------------|------------------------|
| Architecture  | Maintainability (modularity), Reliability | #flexible, #reliable | Reliability            |
| Security      | Security                                  | #secure              | Security               |
| Performance   | Performance Efficiency                    | #efficient           | Performance Efficiency |
| Code Quality  | Maintainability                           | #maintainable        | Maintainability        |
| Test Coverage | (cross-cutting)                           | #reliable            | Reliability            |
| Standards     | Compatibility, Portability                | #suitable, #operable | —                      |
| Usability     | Usability                                 | #usable              | —                      |

**Notable gaps against ISO 25010**:

- Compatibility (interoperability, co-existence) — partially in standards
- Portability (adaptability, installability) — not explicitly covered
- Safety — not covered (domain-specific)
- Functional Suitability (correctness) — implicit, not a dedicated lens

Source: [ISO 25010](https://iso25000.com/index.php/en/iso-25000-standards/iso-25010)

#### Industry Review Frameworks

**Google Engineering Practices** identifies 9 review dimensions: Design,
Functionality, Complexity, Tests, Naming, Comments, Style, Consistency,
Documentation.

**Augment Code's 40-Question Framework** (synthesised from Google, Microsoft,
AWS, OWASP) uses 4 pillars of 10 questions each: Logic & Correctness, Security,
Performance, Maintainability.

**Microsoft Engineering Fundamentals** recommends a two-pass approach: Design
Pass (architecture, patterns, PR description) then Quality Pass (complexity,
naming, error handling, tests).

Sources:

- [Google Engineering Practices](https://google.github.io/eng-practices/review/reviewer/looking-for.html)
- [Augment Code 40-Question Framework](https://www.augmentcode.com/guides/code-review-checklist-40-questions-before-you-approve)
- [Microsoft Engineering Fundamentals](https://microsoft.github.io/code-with-engineering-playbook/code-reviews/process-guidance/reviewer-guidance/)

#### AI-Assisted Review Patterns

The **Specialist-Agent Review** pattern (Qodo, 2026) deploys focused agents
instead of one generalist — directly validating the multi-lens approach:

- Correctness agent: logic bugs, edge cases, error handling, invariants
- Security agent: authz/authn, injection risks, secrets exposure
- Performance agent: hot paths, N+1 queries, algorithmic complexity
- Observability agent, Requirements agent, Standards agent

Key design principles from AI review research:

1. **Category-specific prompts outperform general ones** — separate prompts for
   security, performance, error handling produce higher-quality findings than a
   single general review prompt
2. **Severity-driven triage is essential** — three tiers minimum (blocking,
   recommended, minor)
3. **Context is a required input, not optional** — assembling cross-repo usages,
   historical PRs, and architecture docs before review begins
4. **Noise reduction clauses are critical** — explicit instructions to skip
   trivial style suggestions prevent review fatigue
5. **Attribution-based feedback loops** — tracking accepted/dismissed findings
   enables organic calibration

Sources:

- [Qodo AI Code Review Patterns](https://www.qodo.ai/blog/5-ai-code-review-pattern-predictions-in-2026/)
- [Code Review in the Age of AI (Addy Osmani)](https://addyo.substack.com/p/code-review-in-the-age-of-ai)

#### Review Anti-Patterns Relevant to Lens Design

From AWS, Google, and practitioner literature, key anti-patterns that lens
design should prevent:

| Anti-Pattern                      | Lens Design Implication                                      |
|-----------------------------------|--------------------------------------------------------------|
| Over-focusing on style            | Lenses must prioritise substance; automate style separately  |
| No severity distinction           | Findings must carry severity tiers                           |
| No context awareness              | Lenses must instruct reviewers to explore the codebase       |
| Excessive demands / perfectionism | Lenses must counsel pragmatism and proportionality           |
| Missing actionability             | Findings must include suggested remediation                  |
| Scope creep between reviewers     | Explicit boundary statements prevent overlap                 |
| Priority inversion                | Severity-based sorting ensures critical issues surface first |

Sources:

- [AWS Anti-patterns for Code Review](https://docs.aws.amazon.com/wellarchitected/latest/devops-guidance/anti-patterns-for-code-review.html)
- [Code Review Antipatterns (Simon Tatham)](https://www.chiark.greenend.org.uk/~sgtatham/quasiblog/code-review-antipatterns/)

### Empirical Findings on Review Effectiveness

- **75% of defects found during code review are evolvability defects**
  (maintainability, readability, structure), not functional bugs — validating
  the emphasis on code quality and architecture lenses
- **Checklists reduce extraneous cognitive load** by directing attention to
  specific areas — validating the structured lens approach
- **Reviewer expertise is the strongest predictor** of review effectiveness —
  for AI, this means lens prompts must encode domain expertise deeply
- **Optimal review rate**: 200-400 LOC/hour for humans; AI can handle more but
  effectiveness still degrades with unfocused scope

Sources:

- [What Types of Defects Are Really Discovered in Code Reviews? (Mantyla & Lassenius)](https://www.semanticscholar.org/paper/What-Types-of-Defects-Are-Really-Discovered-in-Code-M%C3%A4ntyl%C3%A4-Lassenius/65e184940d7bd3538c9e59d11da1782d573bae02)
- [Do explicit review strategies improve code review performance? (Springer)](https://link.springer.com/article/10.1007/s10664-022-10123-8)

## Optimal Lens Structure

Based on the convergence of codebase analysis, academic research, quality
frameworks, and industry practice, here is the recommended optimal structure for
a review lens:

### Template

```markdown
---
name: <lens-name>-lens
description: <Lens name> review lens for evaluating <domain summary>. Used by
  review orchestrators — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# <Lens Name> Lens

## Core Responsibilities

[3-4 numbered responsibility groups. Each group has a bold heading and 4-8
bulleted sub-items. Order from most specific/concrete to most holistic/strategic.
Include boundary notes after any responsibility that borders another lens's
domain.]

1. **<Most Concrete Responsibility>**

- <Specific evaluation criterion>
- <Specific evaluation criterion>
- ...

2. **<Intermediate Responsibility>**

- <Specific evaluation criterion>
- ...

3. **<Most Holistic Responsibility>**

- <Specific evaluation criterion>
- ...

[Optional boundary note if needed:]
**Boundary note**: [What another lens covers] is assessed by the [other] lens.
This lens focuses on [what this lens covers] — [distinction].

## Key Evaluation Questions

For each component or change under review, assess:

- **<Dimension>**: <Interrogative question>? <Follow-up question>?
- **<Dimension>**: <Interrogative question>?
- ...

[If the lens has conditionally-applicable sections, group under sub-headings
with applicability notes, e.g., "**API standards** (when API changes are
present):"]

## Important Guidelines

- **Explore the codebase** for existing [domain] patterns and conventions
- **Be pragmatic** — focus on [domain-specific pragmatism guidance]
- **Rate confidence** on each finding — distinguish [high-confidence category]
  from [lower-confidence category]
- **<Domain-specific guideline>** — <explanation>
- ...

## What NOT to Do

- Don't review [list all other lenses] — those are other lenses
- [Specific boundary clarification with another lens, if needed]
- [Domain-specific anti-pattern]
- [Domain-specific anti-pattern]
- Don't [common over-reach for this domain]

Remember: You're evaluating whether [core purpose statement in one sentence].
[Philosophical guidance on what good looks like for this domain].
```

### Design Principles

These principles should guide the creation of any new lens:

#### 1. Single Perspective, Deep Focus (PBR Principle)

Each lens adopts one focused stakeholder perspective. The perspective should be
stated implicitly through the responsibilities and questions rather than
explicitly naming a persona, but the lens author should design with a clear
persona in mind:

| Lens          | Implicit Perspective                              |
|---------------|---------------------------------------------------|
| Architecture  | Systems architect evaluating structural integrity |
| Security      | Attacker probing for vulnerabilities              |
| Performance   | Capacity planner identifying bottlenecks          |
| Code Quality  | Future maintainer reading the code in six months  |
| Test Coverage | Mutation tester trying to break undetected        |
| Standards     | New team member navigating the codebase           |
| Usability     | Consumer using the API for the first time         |

#### 2. Generative Not Passive (PBR Principle)

The most effective review perspectives attempt to produce something, not just
passively scan. Key Evaluation Questions should be phrased to drive generative
thinking:

- "What happens when [dependency] fails?" (generates failure scenarios)
- "What would change if [assumption] changed?" (generates impact analysis)
- "Can an attacker [action]?" (generates attack scenarios)
- "If I changed this operator, would any test fail?" (generates mutation cases)

#### 3. Three to Four Numbered Responsibility Groups

Responsibilities should be grouped into 3-4 numbered categories, ordered from
most concrete to most holistic. Each group should have 4-8 bulleted items. This
provides sufficient structure without being overwhelming.

**Why 3-4**: Aligns with cognitive chunking research. Fewer than 3 means the
lens is too narrow or under-specified. More than 4 means it should probably be
split into two lenses.

#### 4. Explicit Boundary Statements

Every lens must state what it does NOT cover and which other lens covers it.
This prevents:

- Duplicate findings across lenses
- Scope creep during review
- Contradictory recommendations from different lenses

Boundary statements appear in two places:

- **Boundary notes** inline after responsibilities that border another lens
- **What NOT to Do** section listing all other lenses explicitly

When boundaries are ambiguous, include a specific clarifying example
(e.g., the security lens's clarification that security-motivated DoS stays in
security while general algorithmic efficiency is the performance lens).

#### 5. Severity and Confidence Rating

Lenses must instruct reviewers to rate both severity (critical, major, minor,
suggestion) and confidence (high, medium, low). This enables:

- Priority-based triage by the orchestrator
- Appropriate reviewer humility about uncertain findings
- Severity-driven filtering (cap at ~10 inline comments)

#### 6. Pragmatism and Proportionality

Every lens should include guidelines counselling pragmatism. Anti-patterns to
explicitly prevent:

- Perfectionism: "Don't insist on X where simplicity serves better"
- Over-engineering: "Don't recommend adding complexity in the name of best
  practices"
- Style nitpicking: "Don't nitpick style preferences that don't affect Y"
- Premature optimisation: "Only flag issues proportional to expected scale"

#### 7. Codebase Context Awareness

Every lens should instruct the reviewer to explore the existing codebase before
making findings. This prevents false positives from ignoring established
patterns and ensures findings are grounded in the project's actual conventions.

#### 8. Self-Contained Findings

Each finding must be understandable without surrounding context, because
findings from different lenses are aggregated, deduplicated, and presented
together. The finding body format (emoji + lens name + description + impact +
suggestion) enforces this.

#### 9. Closing Mission Statement

The "Remember:" statement serves as a philosophical anchor that guides the
reviewer when the specific rules don't cover a situation. It should capture the
lens's purpose in terms of what "good" looks like, not what "bad" looks like.

### Structural Variation Points

While the template is consistent, lenses vary in these ways:

1. **Number of Core Responsibilities**: 3 is the norm; 4 when a lens has been
   extended (e.g., architecture after resilience addition)

2. **Evaluation Question grouping**: Some lenses have flat question lists
   (architecture, performance, code quality). Others have conditionally-
   applicable sub-groups (standards has "API standards — when API changes are
   present", "Accessibility — when UI changes are involved")

3. **Boundary note complexity**: Simple lenses (test-coverage, usability) have
   one brief boundary note. Complex lenses with many adjacent concerns
   (performance, code-quality) have multiple detailed boundary notes

4. **What NOT to Do specificity**: All lenses list other lenses to avoid. Some
   add domain-specific anti-patterns (performance: "Don't recommend premature
   optimisation"). Some add boundary clarifications with examples (security:
   DoS vs general performance)

### Potential Improvements to Existing Lenses

Based on this research, the existing lenses could be strengthened by:

1. **Making the PBR generative principle more explicit**: Key Evaluation
   Questions could more consistently use generative phrasing ("What happens
   when...", "Can an attacker...", "If I changed..."). Some lenses already do
   this well (architecture, security), others could be improved (standards,
   code quality).

2. **Adding a "Perspective" preamble**: While not strictly necessary (the
   perspective is implicit), a brief sentence stating the lens's perspective
   could help AI reviewers inhabit the role more effectively. For example:
   "Review as if you are the next developer who will maintain this code in six
   months."

3. **Conditional applicability guidance**: The standards lens does this well
   with its sub-grouped questions. Other lenses could benefit from similar
   guidance to avoid irrelevant findings (e.g., performance lens: "Database
   performance — when database interactions are present").

## Architecture Insights

The review system's lens architecture is well-designed for extensibility:

- **Lens-agnostic reviewer agent**: The single `reviewer.md` agent reads its
  lens skill at runtime. Adding a lens requires no agent changes.
- **Shared output formats**: PR and plan review output formats are lens-
  agnostic. No format changes needed for new lenses (only adding the identifier
  to the examples list).
- **Orchestrator-driven selection**: `review-pr` and `review-plan` handle lens
  selection with auto-detect relevance criteria. New lenses need their criteria
  added.
- **Parallel execution**: All lenses run as concurrent agent tasks. Adding a
  lens adds marginal latency.
- **The cost of adding a lens is low**: one new SKILL.md file plus minor edits
  to existing files (boundary statements, orchestrator tables, output format
  identifiers).

The separation between lenses (what to evaluate), output formats (how to
structure output), and orchestrators (how to coordinate) is clean and follows
the single responsibility principle.

## Historical Context

- `meta/research/2026-02-22-review-lens-gap-analysis.md`: Gap analysis that
  identified performance as the primary missing lens and resilience as a
  secondary gap. Recommended extending architecture rather than creating a
  standalone resilience lens.
- `meta/plans/2026-02-23-performance-lens-and-resilience-extension.md`:
  Implementation plan that added the performance lens and resilience to the
  architecture lens, following the gap analysis recommendations.

## Related Research

- `meta/research/2026-02-22-pr-review-agents-design.md`: Design for the
  parallel reviewer agent architecture
- `meta/research/2026-02-22-review-plan-pr-alignment.md`: Alignment between
  plan review and PR review workflows
- `meta/research/2026-02-22-review-lens-gap-analysis.md`: Original gap analysis

## Open Questions

1. **Should lenses include an explicit "Perspective" statement?** The PBR
   research suggests naming the perspective improves review focus. The current
   lenses leave the perspective implicit. Adding a brief perspective sentence
   (e.g., "Review as an attacker probing for vulnerabilities") could improve
   AI reviewer performance.

2. **Should lenses include conditional applicability sub-groups?** The standards
   lens groups questions by conditional applicability (e.g., "when API changes
   are present"). Other lenses use flat question lists. Sub-groups could reduce
   irrelevant findings but add complexity.

3. **Is the existing lens set complete?** Based on ISO 25010 and arc42, the
   main uncovered dimensions are: Compatibility/Interoperability, Portability,
   Functional Suitability (correctness), and Safety. Correctness was
   deliberately excluded (see gap analysis). The others are either too niche or
   adequately covered by existing lenses for most projects.

4. **What is the optimal number of lenses?** PBR research suggests 3
   perspectives provide good coverage for humans. The specialist-agent pattern
   suggests 5-8 is viable for AI. The current 7 seems well-calibrated —
   enough for comprehensive coverage without excessive parallel agents.

5. **Should lenses carry feedback loop metadata?** The attribution-based review
   pattern (Qodo) tracks accepted/dismissed findings per lens to calibrate
   over time. This could be a future enhancement to the orchestrator.
