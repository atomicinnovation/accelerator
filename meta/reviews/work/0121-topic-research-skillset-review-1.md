---
type: "work-item-review"
id: "0121-topic-research-skillset-review-1"
title: "Work Item Review: Topic Research Skillset"
date: "2026-09-06T22:28:38+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0121"
work_item_id: "0121"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: ["clarity", "completeness", "scope"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-09-07T08:01:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Topic Research Skillset

**Verdict:** REVISE

Epic 0121 is unusually well-developed for a draft: every expected section is
substantively filled, the decomposition is explicitly vertical with stated
ordering constraints, and the refinement session's five resolutions are
recorded with traceable provenance. The defects are concentrated in three
places where independently-clear statements are never reconciled — an artifact
contract named as a pillar deliverable but never specified, Slice 5 flagged for
extraction while an acceptance criterion depends on it, and agent topology
(what governs agent count, who writes the synthesis, at what level source
selection binds) left open. Six major findings across three lenses put this
over the REVISE threshold, but every one is a documentation fix inside the work
item rather than a rethink of the underlying design.

### Cross-Cutting Themes

- **The artifact contract is referenced but never defined** (flagged by:
  clarity, completeness) — Requirements theme 1 names "a stable
  brief/topic/synthesis artifact contract" as one of two infrastructure
  pillars, and acceptance criterion 1 requires files "with the defined
  frontmatter and sections". No section of the work item enumerates that
  frontmatter or those sections. Only `manifest.md` gets partial treatment in
  Technical Notes; `brief.md`, per-topic files, `synthesis.md` and the closed
  set of `research_kind` values are absent. Because the contract is explicitly
  meant to be *stable* and consumed by later research skills, divergent
  invention is expensive to unwind once topic files become immutable.

- **Slice 5's in/out status leaves the epic boundary undetermined** (flagged
  by: scope, completeness) — Slice 5 calls itself a "Candidate for extraction
  as its own work item", yet acceptance criterion 10 requires exactly its
  deliverable. Extract it and the epic cannot satisfy its own criteria; retain
  it and closure gates on an undecided API shape plus a prototyping step
  independent of the research engine. Compounding this, no slice is linked to a
  child work item and the epic never states whether slices will be split out or
  worked directly.

- **Theme 4's source model is both unbounded and inconsistently named** (flagged
  by: scope, clarity) — theme 4 keeps "other sensible external sources" in
  scope, but no slice delivers them and no criterion verifies them. The same
  theme's tiering mechanism is called credibility tiers, reputation-tiered
  citations, citation/source tiers and reputation-only tiering across five
  sections, with nothing stating these are one concept.

### Findings

#### Critical

None.

#### Major

- 🟡 **Clarity + Completeness**: The artifact contract is named as a core
  deliverable but never specified
  **Location**: Requirements: High-level goals and themes (item 1) /
  Acceptance Criteria 1
  Theme 1 names "a stable brief/topic/synthesis artifact contract" as an
  infrastructure pillar and criterion 1 requires "the defined frontmatter and
  sections", but no section defines them; the definite article points at a
  definition the reader cannot consult. The trailing "interactively scoped"
  also dangles — it is unclear whether it modifies the brief, the directory, or
  the act of production, and no actor for the interaction is named.

- 🟡 **Clarity**: Agent count governed by two uncoordinated controls
  **Location**: Acceptance Criteria 2
  Criterion 2 says `conduct` spawns agents "(count scaled by breadth and the
  effort-scaling rubric)" without stating how the two interact — whether
  `breadth` is a hard cap, a starting point the rubric may adjust, or only a
  default the rubric overrides. Two implementers can satisfy the criterion with
  opposite behaviours, and the cost guard mitigating the ~15× token cost may be
  silently bypassed.

- 🟡 **Scope**: Slice 5 is flagged for extraction yet load-bearing in the
  acceptance criteria
  **Location**: Requirements: Initial stories — Slice 5
  Slice 5 is described as a "Candidate for extraction as its own work item"
  while acceptance criterion 10 requires its deliverable, leaving the epic's
  completion boundary undefined. Slice 5 also carries the epic's sole
  unresolved Open Question and the only pre-implementation prototyping step.

- 🟡 **Scope**: Slice 1 "walking skeleton" bundles two separately shippable
  increments across four toolchains
  **Location**: Requirements: Initial stories — Slice 1
  Slice 1 enumerates roughly a dozen establishing deliverables spanning config
  key, four templates, per-skill wiring, manifest root, generic agent, web
  profile, two subcommands, orchestrator prompt, full visualiser doc-type
  registration with VR baselines on two platforms, and nested-manifest indexer
  integration. It fuses the research engine and the visualiser registration,
  which could each be delivered, demoed and rolled back independently.

- 🟡 **Clarity**: Source selection is stated at three different granularities
  **Location**: Acceptance Criteria 5 and 6 / Slice 3
  Slice 3 places the web-versus-academic choice on the brief ("the brief
  declares a source preference"), criterion 6 on the round ("Academic rounds
  draw on OpenAlex and arXiv"), and criterion 5 on the spawn ("specialised for
  web vs academic purely by injected profile"). Whether a single round can fan
  out mixed researchers, or a whole brief is locked to one source family, is
  left to inference — and this binding level propagates into the profile
  contract, the brief template and the orchestrator prompt.

- 🟡 **Clarity**: "The synthesiser" is an actor that appears nowhere else
  **Location**: Technical Notes: Citations
  The Citations note says "the synthesiser reads the tagged topic files and
  carries citations forward", introducing an actor named nowhere else, while
  criterion 2 attributes writing `synthesis.md` to `conduct` itself. Since the
  same paragraph insists there is "no separate citation agent", the reader
  cannot tell whether the synthesiser is a fourth agent, a mode of the
  `researcher`, or the orchestrator writing inline.

#### Minor

- 🔵 **Scope**: Theme 4 keeps an unbounded source category in scope that no
  slice or criterion covers
  **Location**: Requirements: High-level goals and themes (item 4)
  Theme 4 places "other sensible external sources" inside scope, but the slice
  preamble excludes them from all five slices and no criterion references them.
  The epic carries a residual with no natural stopping point, so closure can be
  contested by anyone reading the themes rather than the slices.

- 🔵 **Clarity**: One source-quality concept is named four different ways
  **Location**: Requirements: High-level goals and themes (theme 4)
  "Credibility tiers", "reputation-tiered citations", "citations/source tiers",
  "citation-tier ideas" and "reputation-only tiering" appear across five
  sections with nothing stating they are the same mechanism. "Reputation-only"
  in particular reads as a narrowing of something broader.

- 🔵 **Clarity**: `manifest.md`'s `primary` pointer is read by Slice 5 but no
  slice says who writes it
  **Location**: Requirements: Initial stories (Slice 2)
  Slice 5 and criterion 10 both depend on the `primary` pointer, but Slice 2's
  maintained-field list ("status, round count and topic count") and criterion 2
  ("status and counts") both omit it. Whether it is written at `brief` time,
  flipped by `conduct` after round one, or derived by the reader is unstated —
  precisely the cross-slice dependency vertical slicing should make explicit.

- 🔵 **Clarity**: "The corpus" is undefined and appears to conflict with the
  append-only round log
  **Location**: Acceptance Criteria 3
  Criterion 3 ends "the corpus reads as a single dossier (no round-by-round
  narration)", but if "the corpus" means everything under the slug directory it
  contradicts the append-only round log required by theme 3, Slice 2 and
  criterion 2. A reader working from the criteria alone may apply the
  anti-changelog rule to the brief and destroy the round history.

- 🔵 **Clarity**: Unbounded qualifiers leave two slice deliverables
  undetermined
  **Location**: Requirements: Initial stories (Slices 1 and 2)
  Slice 1 promises "sensible hardcoded depth/breadth defaults (depth 1)" but
  the breadth default value appears nowhere, though criterion 8 requires that
  "defaults are documented". Slice 2 promises "any visualiser grouping needed",
  leaving both the existence and shape of that deliverable to the implementer.

- 🔵 **Completeness**: Slices are not linked to child work items, and Slice 5's
  status is unresolved
  **Location**: Requirements: Initial stories
  No slice is linked to a child work item, frontmatter carries no child
  references, and the epic never says whether slices will be split out or
  worked directly from this document. A reader cannot tell whether the epic is
  ready to be worked, awaiting decomposition, or partially decomposed.

- 🔵 **Completeness**: Context explains design provenance but not the
  motivating problem
  **Location**: Context
  Context covers where the design came from, which conventions it conforms to,
  and that the area is greenfield — but never states the problem being solved.
  For a five-slice epic with acknowledged ~15× token cost, a reader has no
  basis to judge whether the investment is warranted or which slices matter
  most if it must be trimmed.

- 🔵 **Completeness**: Acceptance criteria are not attributed to slices
  **Location**: Acceptance Criteria
  Eleven criteria are listed flat while the five slices are declared
  "independently shippable, demoable, and testable" with negotiable ordering.
  Without a mapping, a team cannot tell which criteria gate shipping an
  individual slice, undermining the independent shippability the epic is built
  around.

- 🔵 **Completeness**: Frontmatter status and priority appear provisional after
  refinement
  **Location**: Frontmatter: status, priority
  `status: draft` survives a 2026-09-06 refinement that resolved five open
  questions, added a fifth slice and produced eleven criteria. `priority:
  medium` is flagged twice in the body as a default that should be adjusted — a
  self-declared unresolved value that survived refinement.

- 🔵 **Scope**: Slices 2 and 3 each pair a core behaviour with a separately
  shippable enhancement
  **Location**: Requirements: Initial stories — Slices 2 and 3
  Slice 2 pairs idempotent multi-round accretion with the user-facing `expand`
  subcommand; the accretion machinery is a prerequisite for `expand` but not
  the reverse. Slice 3 pairs two new academic providers with inline
  reputation-tier tagging, a cross-cutting concern applying equally to the web
  sources already delivered in Slice 1.

#### Suggestions

- 🔵 **Completeness**: No criterion addresses the quality of the research
  output itself
  **Location**: Acceptance Criteria
  All eleven criteria describe structural and mechanical outcomes — files in
  the right paths, doc type registered, flags resolving in order, `mise run
  check` passing. None addresses whether the produced dossier is a good
  research artefact, though the stated user value is a "thorough,
  citation-backed knowledgebase".

- 🔵 **Clarity**: Several abbreviations and internal terms are used before or
  without definition
  **Location**: Requirements: Initial stories (Slice 1)
  "VR baselines" is only expanded in criterion 10; "the 3-tier template
  override" is used four times without enumerating the tiers; "card" is used
  for what is elsewhere a library entry; "INVEST-aligned" is unexpanded; the
  "`mailto`/User-Agent polite pool" is only partly glossed.

- 🔵 **Clarity**: "Claude Design" is undefined and the prototype's status is
  ambiguous
  **Location**: Requirements: Initial stories (Slice 5)
  The tag "*(prototyped in Claude Design)*" reads as work already done, while
  "Prototyped first as an extension to the Claude Design prototype, then
  implemented" reads as work inside the slice. The definite article asserts an
  artefact the reader cannot locate, and the ambiguity changes the slice's size
  and its viability as the extracted work item the same bullet proposes.

- 🔵 **Clarity**: Resolved questions are restated in two sections and sit under
  a heading that implies they are open
  **Location**: Open Questions
  The second paragraph of Open Questions and the final Drafting Notes bullet
  narrate the same five resolutions in near-identical prose. Two copies can
  drift apart on the next edit, and the heading gives a skimming reader a mixed
  signal about what is still undecided.

### Strengths

- ✅ Epic decomposition is exemplary: five explicitly vertical slices, each
  cutting skill → agent → template → artifact → visualiser, with the Drafting
  Notes recording the deliberate rejection of a horizontal "infrastructure"
  story — a common epic anti-pattern avoided on purpose.
- ✅ Child ordering constraints are explicit (Slice 1 foundational, Slices 2–4
  reorderable, Slice 5 dependent on Slices 1–2), so the epic can be sequenced
  without discovering hidden coupling mid-flight.
- ✅ Coined and borrowed vocabulary is defined before use — "anti-changelog",
  the effort-scaling rubric with its 1 / 2–4 / 10+ numbers, and the depth ×
  breadth parameterisation are introduced in Context and reused consistently.
- ✅ Depth and breadth have explicit, non-overlapping definitions plus a stated
  precedence order (flag > config > hardcoded default), removing the most
  common ambiguity in tunable-parameter work items.
- ✅ The reusable-infrastructure claim is validated inside the epic rather than
  deferred: two profiles (web in Slice 1, academic in Slice 3) exercise the
  injectable seam, and criterion 5 asserts it directly.
- ✅ Out-of-scope items are named repeatedly and consistently — the ideation
  skillset, consumer research skills, internal sources, Crossref, Semantic
  Scholar, the dedicated citation pass — each designed as an additive drop-in
  that keeps the epic boundary stable rather than merely postponed.
- ✅ Referents are overwhelmingly file- and path-level, so most statements
  resolve to a checkable artefact rather than an abstraction.
- ✅ Technical Notes carry substantial implementation grounding (visualiser
  checklist, indexer model, depth/breadth semantics, provider choices, citation
  strategy), so a planner inherits the reasoning, not just the conclusions.
- ✅ Assumptions pin interpretive choices a reader would otherwise guess, and
  Drafting Notes make the provenance of every scope decision traceable —
  including which calls were made at the user's direction.
- ✅ The one overlapping neighbour (`0056`) is scoped away with a stated reason,
  so the epic does not depend on a parallel thread completing simultaneously.

### Recommended Changes

1. **Specify the artifact contract, or name it as a Slice 1 planning
   deliverable** (addresses: "The artifact contract is named as a core
   deliverable but never specified")
   Add an "Artifact contract" subsection under Requirements enumerating, at
   least at field-name level, the frontmatter and required sections for
   `manifest.md`, `brief.md`, per-topic files and `synthesis.md`, plus the
   closed set of `research_kind` values. If that is genuinely planning work,
   say so and reword criterion 1 to stop pointing at a definition that does not
   exist. Also rewrite the trailing "interactively scoped" to name who scopes
   the brief and with whom.

2. **Settle Slice 5's in/out status now** (addresses: "Slice 5 is flagged for
   extraction yet load-bearing"; "Slices are not linked to child work items")
   Either extract Slice 5 into its own work item and rewrite criterion 10 to
   require only library-entry browsability (which Slice 1 already delivers), or
   keep it in scope and delete the extraction note. While there, state the
   decomposition intent for all five slices — whether each becomes a child
   story before implementation — and record child ids in frontmatter as they
   are created.

3. **State how `breadth` and the effort-scaling rubric interact** (addresses:
   "Agent count governed by two uncoordinated controls")
   Say explicitly in criterion 2 whether `breadth` is an upper bound the rubric
   may only reduce, or whether the rubric applies only when `breadth` is unset,
   and mirror that wording in the Slice 4 resolution order. This is the cost
   guard for the epic's headline token risk, so it should not be inferable in
   two directions.

4. **Pick one granularity for source selection and one name for the tiering
   scheme** (addresses: "Source selection is stated at three different
   granularities"; "One source-quality concept is named four different ways")
   State once — e.g. the brief declares a set of source profiles and the
   orchestrator assigns one profile per spawned agent, so a round may be mixed
   — and rephrase "Academic rounds" accordingly. Separately, choose "reputation
   tier", define it once in theme 4 including what "reputation-only" excludes,
   and use it everywhere.

5. **Name the component that rewrites `synthesis.md`** (addresses: "The
   synthesiser is an actor that appears nowhere else")
   Replace "the synthesiser" with the named component — orchestrator, a spawned
   agent, or a `researcher` with a synthesis profile — and if it is a distinct
   agent, add it to the Slice 1 and Slice 2 deliverable lists.

6. **Split Slice 1 into engine and visualiser halves** (addresses: "Slice 1
   walking skeleton bundles two separately shippable increments")
   Split into 1a (skill + agent + web profile + templates + config key,
   demoable as artifacts on disk) and 1b (umbrella doc-type registration,
   nested-manifest indexing, VR baselines), sequencing 1b immediately after 1a
   so the end-to-end vertical demo still lands early. Consider the same
   treatment for Slices 2 and 3, where `expand` and reputation tagging are each
   separable from their slice's core behaviour.

7. **Close the remaining scope and referent gaps** (addresses: "Theme 4 keeps an
   unbounded source category"; "`primary` pointer has no writer"; "The corpus is
   undefined"; "Unbounded qualifiers")
   Narrow theme 4 to web and academic and move other source families to the
   out-of-scope list. Add `primary` to the fields Slice 2 keeps in sync, or say
   Slice 1's `brief` writes it and `conduct` flips it. Replace "the corpus" with
   the artefacts the anti-changelog rule governs, explicitly exempting
   `brief.md`'s round log. State the numeric breadth default, and either name
   Slice 2's visualiser grouping or say none is needed until Slice 5.

8. **Refresh frontmatter and add slice attribution** (addresses: "Frontmatter
   status and priority appear provisional"; "Acceptance criteria are not
   attributed to slices"; "Context explains provenance but not the motivating
   problem")
   Advance `status` past `draft` or say what still blocks it, and settle
   `priority` so the "adjust if higher-urgency" caveats can be removed.
   Annotate each criterion with its slice. Add a short opening paragraph to
   Context describing how external-topic research is done today and what the
   knowledgebase unlocks, so prioritisation is possible from the work item
   alone.

## Per-Lens Results

### Clarity

**Summary**: This is a dense, unusually well-disciplined epic: most coined terms
(anti-changelog, effort-scaling rubric, depth × breadth) are defined in Context
before they are used, artefacts are named by concrete path, and the Technical
Notes explicitly record resolutions rather than leaving them implicit. The
clarity weaknesses are concentrated in places where two independently-clear
statements are never reconciled — breadth versus the effort-scaling rubric as
competing controls on agent count, source selection stated at brief-, round- and
agent-level in three different sections, and a `manifest.primary` pointer that
Slice 5 reads but no slice writes. A second cluster is terminology drift for one
concept (credibility / reputation / citation / source tiers) and dangling
referents in the Acceptance Criteria ("the defined frontmatter and sections",
"the corpus").

**Strengths**:

- Coined and borrowed vocabulary is defined before use: "anti-changelog", the
  effort-scaling rubric (with its 1 / 2–4 / 10+ numbers) and the depth × breadth
  parameterisation are all introduced in Context and then used consistently in
  Requirements and Acceptance Criteria.
- Depth and breadth are given explicit, non-overlapping definitions ("flat
  per-round parallel fan-out" versus "optional intra-round auto-recursion
  defaulting to 1") plus a stated precedence order (flag > config > hardcoded
  default), removing the most common source of ambiguity in tunable-parameter
  work items.
- Referents are overwhelmingly file- and path-level (`meta/research/topics/<slug>/`,
  `manifest.md`, `synthesis.md`, `brief.md`), so most statements resolve to a
  checkable artefact rather than an abstraction.
- The Assumptions section pins interpretive choices a reader would otherwise
  have to guess (YAML frontmatter over Sagan's bold-markdown headers,
  external-only sources, reuse of the `configure` dispatch precedent, why
  priority is Medium).
- Resolved questions are recorded with both the resolution and where it now
  lives ("See the slices and Technical Notes"), so a reader can trace a decision
  rather than re-litigate it.
- The Initial-stories preamble explicitly states which theme content is excluded
  from the slices and why, pre-empting a scope misreading that would otherwise
  be silent.

**Findings**:

- **major / high** — *Agent count governed by two uncoordinated controls
  (breadth and the effort-scaling rubric)* — Location: Acceptance Criteria.
  Acceptance criterion 2 says `conduct` "spawns parallel `researcher` agents
  (count scaled by breadth and the effort-scaling rubric)", but the work item
  never states how the two interact: `breadth` is defined in Slice 4 as "the
  flat per-round parallel fan-out" resolved as flag > config > hardcoded
  default, while the effort-scaling rubric from Context prescribes "1 agent for
  a simple topic, 2–4 for comparisons, 10+ for broad subjects". If a user sets
  `--breadth 4` on a broad subject the rubric implies 10+, and nothing says
  which wins, whether breadth is a hard cap, a starting point the rubric may
  adjust, or only a default the rubric overrides. **Impact**: Two implementers
  can satisfy the criterion with opposite behaviours, and the cost guard that
  both Slice 1 and the Technical Notes rely on to mitigate the ~15× token cost
  may be silently bypassed. **Suggestion**: State the relationship explicitly in
  the criterion — e.g. whether `breadth` is an upper bound the rubric may only
  reduce, or whether the rubric applies only when `breadth` is unset — and
  mirror the wording in the Slice 4 resolution order.

- **major / medium** — *Source selection stated at three different granularities
  (brief, round, agent)* — Location: Acceptance Criteria. Three statements place
  the web-versus-academic choice at different levels: Slice 3 says "the brief
  declares a source preference the generic `researcher` honours" (per-brief),
  acceptance criterion 6 says "Academic rounds draw on OpenAlex and arXiv"
  (per-round), and acceptance criterion 5 says the agent "is specialised for web
  vs academic purely by injected profile" (per-spawn). Whether a single round
  can fan out mixed web and academic researchers, or whether a whole brief is
  locked to one source family, is left for the reader to infer. **Impact**: The
  injectable-profile seam is the epic's central reusability claim, so an
  ambiguous binding level propagates into the profile contract, the brief
  template's frontmatter and the orchestrator prompt. **Suggestion**: Pick one
  granularity and use it consistently — e.g. state that the brief declares a set
  of source profiles and the orchestrator assigns one profile per spawned agent,
  so a round may be mixed — and rephrase "Academic rounds" accordingly.

- **major / medium** — *"The synthesiser" is an actor that appears nowhere else
  in the work item* — Location: Technical Notes: Citations. The Citations note
  says "the synthesiser reads the tagged topic files and carries citations
  forward as it rewrites `synthesis.md`", introducing an actor named nowhere
  else; the named components elsewhere are the `research-topic` skill, the
  `conduct` orchestrator prompt and the generic `researcher` agent, and
  acceptance criterion 2 attributes writing `synthesis.md` to `/research-topic
  conduct` itself. Because the same paragraph is at pains to say there is "no
  separate citation agent" (and criterion 5 that there is no per-source agent),
  a reader cannot tell whether the synthesiser is a fourth spawned agent, a mode
  of the `researcher`, or just the orchestrator doing the write inline.
  **Impact**: In an epic whose whole architecture is about which component does
  what in which context, an unnamed writer of the most important output artefact
  leaves the agent topology undetermined. **Suggestion**: Replace "the
  synthesiser" with the named component that performs the rewrite (orchestrator,
  a spawned agent, or a `researcher` with a synthesis profile) and, if it is a
  distinct agent, add it to the Slice 1/Slice 2 deliverable lists.

- **minor / medium** — *`manifest.md`'s `primary` pointer is read by Slice 5 but
  no slice says who writes it* — Location: Requirements: Initial stories (Slice
  2). Slice 5 and acceptance criterion 10 both depend on `manifest.md`'s
  `primary` pointer ("reads `manifest.md`'s `primary` pointer to choose the
  default sub-document (brief before the first round, synthesis after)"), but
  Slice 2 enumerates exactly which manifest fields are maintained —
  "`conduct`/`expand` keep `manifest.md`'s status, round count and topic count
  in sync" — and acceptance criterion 2 likewise says "updates `manifest.md`'s
  status and counts". `primary` is absent from both lists, so it is unclear
  whether it is written once at `brief` time, flipped by `conduct` after the
  first round, or derived by the reader. **Impact**: The reader must guess
  whether Slice 5's default-sub-document behaviour needs producer-side work in
  Slices 1–2, which is precisely the kind of cross-slice dependency the vertical
  slicing was meant to make explicit. **Suggestion**: Add `primary` to the field
  list Slice 2 keeps in sync (or state in Slice 1 that `brief` writes it and
  `conduct` flips it after the first round) so the field has a named writer as
  well as a named reader.

- **minor / high** — *"the defined frontmatter and sections" points at a
  definition that does not exist in the work item* — Location: Acceptance
  Criteria. Acceptance criterion 1 requires `brief.md` and `manifest.md` to be
  produced "with the defined frontmatter and sections, interactively scoped".
  The definite article implies a definition the reader can consult, but no
  section of the work item enumerates the frontmatter fields or section headings
  and no link is given (the Technical Notes list manifest *metadata* — title,
  status, round/topic counts, `primary` — but never say those are the
  frontmatter, nor cover `brief.md` at all). The trailing "interactively scoped"
  also dangles: it is unclear whether it modifies the brief, the whole
  directory, or the act of production, and no actor for the interaction is
  named. **Impact**: The reader cannot tell what "the defined" refers to, so the
  criterion cannot be interpreted the same way by two people planning Slice 1.
  **Suggestion**: Either name where the frontmatter and section contract lives (a
  template deliverable in Slice 1, or an existing convention document) or state
  the required fields inline, and rewrite the trailing clause to name who is
  scoping the brief and with whom.

  *(Merged into the aggregate major finding with the completeness lens's
  "stable artifact contract" finding.)*

- **minor / medium** — *"the corpus" is undefined and appears to conflict with
  the append-only round log* — Location: Acceptance Criteria. Acceptance
  criterion 3 ends "the corpus reads as a single dossier (no round-by-round
  narration)", but "the corpus" is never defined. If it means everything under
  `meta/research/topics/<slug>/` it directly contradicts the append-only round
  log required by requirement theme 3, Slice 2 and criterion 2 ("appends a
  proposed next round to the brief"), which is inherently round-by-round; only
  the Context bullet on Sagan hints that the log lives in the brief while the
  anti-changelog rule applies to topic files and the synthesis. **Impact**: A
  reader working from the Acceptance Criteria alone sees two requirements that
  cannot both hold, and may implement the anti-changelog rule over the brief and
  destroy the round history. **Suggestion**: Name the artefacts the
  anti-changelog rule governs (e.g. "the topic files and `synthesis.md` read as
  a single dossier; `brief.md` retains the append-only round log") rather than
  relying on the undefined collective noun.

- **minor / high** — *One source-quality concept is named four different ways* —
  Location: Requirements: High-level goals and themes (theme 4). What appears to
  be a single mechanism is referred to as "credibility tiers" (theme 4),
  "reputation-tiered citations" and "inline reputation-tier tagging" (Slice 3),
  "citations/source tiers" (Slice 3, template work), "citation-tier ideas"
  (Context) and "reputation-only tiering" (Open Questions). Nothing states that
  these are the same thing, and "reputation-only" in particular reads as a
  deliberate narrowing of something broader, which invites the reading that
  credibility tiers and reputation tiers are distinct concepts. **Impact**: A
  reader cannot tell whether the epic delivers one tiering scheme or several,
  which affects the topic-file format, the synthesis carry-forward and the
  visualiser rendering. **Suggestion**: Choose one term (e.g. "reputation
  tier"), define it once in Context or theme 4 including what "reputation-only"
  excludes, and use it in every subsequent mention.

- **minor / medium** — *Unbounded qualifiers leave two Slice deliverables
  undetermined* — Location: Requirements: Initial stories (Slices 1 and 2). Two
  slice deliverables are stated as properties rather than artefacts: Slice 1
  promises "sensible hardcoded depth/breadth defaults (depth 1)" — the depth
  default is given but the breadth default value appears nowhere in the work
  item, even though acceptance criterion 8 requires that "defaults are
  documented" — and Slice 2 promises "any visualiser grouping needed to show
  round/topic structure", which leaves both the existence and the shape of that
  deliverable to the implementer's judgement. **Impact**: Neither statement has a
  single interpretation, so the boundary between Slice 1 and Slice 4, and
  between Slice 2 and Slice 5, shifts depending on who reads it.
  **Suggestion**: State the numeric breadth default alongside `depth 1`, and
  either name the concrete grouping Slice 2 delivers or say explicitly that no
  grouping is required until Slice 5.

- **suggestion / medium** — *Several abbreviations and internal terms are used
  before (or without) definition* — Location: Requirements: Initial stories
  (Slice 1). A handful of terms are used ahead of any definition or with none at
  all: "VR baselines" appears in Slice 1 and the Technical Notes and is only
  expanded as "visual-regression baselines" later, in acceptance criterion 10;
  "the 3-tier template override" / "3-tier override" is used four times without
  ever enumerating the three tiers; "no half-written card" (Technical Notes,
  indexer model) uses "card" for what is elsewhere called a library entry;
  "INVEST-aligned" (Drafting Notes) is unexpanded; and "the `mailto`/User-Agent
  polite pool" is only partly glossed by the parenthetical about anonymous-pool
  `429`s. **Impact**: Individually trivial, but collectively they force a reader
  who has not worked on the visualiser or the config system to pause and
  reconstruct meaning in the densest part of the work item. **Suggestion**:
  Expand each on first use or link it once (e.g. name the three override tiers
  the first time, expand VR in Slice 1, and use "library entry" consistently
  instead of "card").

- **suggestion / medium** — *"Claude Design" is undefined and the prototype's
  status is ambiguous* — Location: Requirements: Initial stories (Slice 5).
  Slice 5 is tagged "*(visualiser-only; prototyped in Claude Design)*" and its
  body says "Prototyped first as an extension to the Claude Design prototype,
  then implemented against the umbrella doc type". "Claude Design" is named
  nowhere else, is not in the References, and the two phrasings pull in
  different directions: the past participle in the tag reads as work already
  done, while "Prototyped first ... then implemented" reads as work inside the
  slice, and "the Claude Design prototype" (definite article) asserts an
  existing artefact the reader cannot locate. **Impact**: A reader cannot tell
  whether Slice 5 includes prototyping effort or merely consumes an existing
  prototype, which changes the slice's size and its viability as the extracted
  work item the same bullet proposes. **Suggestion**: Add a one-line gloss or
  link for Claude Design and the existing prototype, and use a single tense —
  either "extends the existing X prototype" or "includes prototyping in X
  first".

- **suggestion / medium** — *Resolved questions are restated in two sections and
  sit under a heading that implies they are open* — Location: Open Questions.
  The second paragraph of Open Questions and the final bullet of Drafting Notes
  both narrate the same five refinement resolutions in near-identical prose
  (umbrella doc type, `manifest.md` aggregate root, OpenAlex + arXiv,
  depth/breadth semantics, inline citation tagging). Placing resolved decisions
  under a heading named "Open Questions" also invites a skimming reader to treat
  them as still open, especially since the genuinely open item above them is
  about Slice 5. **Impact**: Two copies of the same decision record can drift
  apart on the next edit, and a reader scanning for what is still undecided gets
  a mixed signal from the section heading. **Suggestion**: Keep the resolution
  narrative in one place (Drafting Notes, where the decision history belongs)
  and leave Open Questions containing only the still-open Slice 5 API-shape
  question, with a one-line pointer to where the resolved ones are recorded.

### Completeness

**Summary**: This epic is unusually well populated for a draft: every expected
section (Summary, Context, Requirements, Acceptance Criteria, Open Questions,
Dependencies, Assumptions, Technical Notes, References) is present and
substantively filled, and the epic-specific requirement — a decomposition
strategy — is met by five explicitly vertical, independently shippable slices
with stated ordering constraints. Frontmatter carries all required fields with
recognised values. The residual gaps are informational rather than structural:
the "stable artifact contract" the epic names as a first-class deliverable is
never specified anywhere in the document, the slices are not linked to (or
scheduled to become) child work items, and the Context explains design
provenance rather than the motivating problem.

**Strengths**:

- Epic decomposition is exemplary: five vertical slices, each described as
  independently shippable and cutting skill → agent → template → artifact →
  visualiser, with explicit statements about which are foundational, which are
  reorderable, and what is deliberately excluded from them.
- Open Questions is honestly maintained — five drafting-time questions are
  recorded as resolved with their resolutions summarised and cross-referenced
  into the slices and Technical Notes, and the single genuinely open question is
  scoped to a decision point ("decide during Slice 5 planning").
- Acceptance Criteria contains eleven specific, concrete bullets that
  collectively cover every one of the six high-level requirement themes.
- Assumptions and Dependencies are both populated with real content
  (external-only sources, reuse of the `configure` dispatch precedent,
  relationship to work item 0056, out-of-scope future consumers) rather than
  left as empty headings.
- Technical Notes carry substantial implementation-grounding — visualiser
  doc-type checklist, indexer model, depth/breadth semantics, academic provider
  choices, citation strategy — so a planner inherits the reasoning, not just the
  conclusions.
- Drafting Notes make the provenance of scope decisions traceable, including
  which calls were made at the user's direction and which are flagged for a
  reviewer to revisit.

**Findings**:

- **major / medium** — *The "stable artifact contract" named as a core
  deliverable is never specified* — Location: Requirements: High-level goals and
  themes (item 1) / Acceptance Criteria. Epic 0121 names "a stable
  brief/topic/synthesis artifact contract" as one of two pillars of the reusable
  infrastructure it delivers, and the first acceptance criterion requires
  `brief` to produce files "with the defined frontmatter and sections" — but no
  section of the work item defines that frontmatter or those sections. Only
  `manifest.md` gets partial treatment (title, status, round/topic counts,
  `primary` pointer, in Technical Notes); the brief, per-topic and synthesis
  document shapes, and the permitted values of the `research_kind`
  discriminator, are absent. **Impact**: The acceptance criterion points at a
  definition that does not exist, so an implementer must invent the contract;
  because the contract is explicitly meant to be *stable* and consumed by later
  research skills, divergent invention is expensive to unwind after topic files
  become immutable. **Suggestion**: Add a short "Artifact contract" subsection
  under Requirements (or Technical Notes) enumerating, at least at field-name
  level, the frontmatter and required sections for `manifest.md`, `brief.md`,
  per-topic files and `synthesis.md`, plus the closed set of `research_kind`
  values — or state explicitly that the contract is a Slice 1 planning
  deliverable and reword the acceptance criterion accordingly.

  *(Merged with the clarity lens's "the defined frontmatter and sections"
  finding into the aggregate major finding.)*

- **minor / medium** — *Slices are described in prose but not linked to child
  work items, and Slice 5's status is unresolved* — Location: Requirements:
  Initial stories (vertical slices). Epic 0121 decomposes into five named slices
  in prose, but no slice is linked to a child work item, the frontmatter carries
  no child references (only `relates_to: work-item:0056`), and the work item
  never says whether the slices will be split out or worked directly from this
  document. Slice 5 compounds this by describing itself as a "Candidate for
  extraction as its own work item" — an unresolved in-or-out-of-this-epic
  decision that is not recorded in Open Questions. **Impact**: A reader cannot
  tell whether the epic is ready to be worked, awaiting decomposition, or
  partially decomposed already, and the ambiguity over Slice 5 means the epic's
  own boundary — and therefore when it is done — is undetermined.
  **Suggestion**: State the decomposition intent explicitly (e.g. "each slice
  becomes a child story before implementation starts"), record child work-item
  ids in frontmatter as they are created, and promote the Slice 5 extraction
  question into Open Questions with a decision point.

- **minor / medium** — *Context explains design provenance but not the
  motivating problem* — Location: Context. The Context section of epic 0121
  covers where the design came from (a survey of Sagan, Anthropic's multi-agent
  system, dzhng/deep-research, superpowers), which Accelerator conventions it
  conforms to, and that the area is greenfield — but it never states the problem
  being solved or the opportunity being captured. The only motivation in the
  whole document is the Summary's trailing "so that I can quickly build a
  thorough, citation-backed knowledgebase on a subject". **Impact**: For a
  five-slice epic with acknowledged high token cost (~15× a chat per the
  Anthropic reference), a reader has no basis on which to judge whether the
  investment is warranted, or which slices matter most if the epic must be
  trimmed. **Suggestion**: Add a short opening paragraph to Context describing
  the current pain or workaround (how external-topic research is done today
  without this skillset), and what value the knowledgebase unlocks — enough that
  a prioritisation decision can be made from the work item alone.

- **minor / medium** — *Acceptance criteria are not attributed to slices* —
  Location: Acceptance Criteria. Epic 0121 lists eleven acceptance criteria as a
  single flat list while explicitly stating that its five slices are
  "independently shippable, demoable, and testable" with negotiable ordering.
  Nothing in the document maps criteria to slices — for example, the criterion
  covering `--breadth`/`--depth` resolution clearly belongs to Slice 4 and the
  detail-page criterion to Slice 5, but readers must reconstruct this by
  cross-referencing the slice prose. **Impact**: Without the mapping, a team
  cannot tell which criteria gate the shipping of an individual slice,
  undermining the independent-shippability the epic is designed around and
  risking a slice being called done against the wrong bar. **Suggestion**:
  Annotate each acceptance criterion with the slice it belongs to (e.g. a
  trailing `(Slice 3)`), or group the criteria under per-slice subheadings.

- **minor / medium** — *Frontmatter status and priority appear provisional after
  refinement* — Location: Frontmatter: status. Epic 0121 carries `status: draft`
  although `last_updated` is 2026-09-06 and the body records a refinement
  session on that date which resolved five of six open questions, added a fifth
  slice, and produced eleven acceptance criteria. Separately, `priority: medium`
  is flagged twice in the body (Assumptions and Drafting Notes) as a default
  that was set for lack of information and should be adjusted — a self-declared
  unresolved value that survived refinement. **Impact**: Stale or
  admittedly-provisional frontmatter misroutes the work item in any status- or
  priority-driven view and understates how ready the epic actually is.
  **Suggestion**: Advance `status` to the appropriate post-refinement value (or
  state in the body what still blocks it), and either confirm `priority: medium`
  as a deliberate decision or set it, removing the "adjust if higher-urgency"
  caveats once settled.

- **suggestion / medium** — *No criterion addresses the quality of the research
  output itself* — Location: Acceptance Criteria. All eleven acceptance criteria
  in epic 0121 describe structural and mechanical outcomes — files written to
  the right paths, the doc type registered in the visualiser, flags resolving in
  the right precedence order, `mise run check` passing. None addresses whether
  the produced dossier is actually a good research artefact, even though the
  epic's stated user value is a "thorough, citation-backed knowledgebase" and it
  invests a whole slice in source rigour and reputation tiering. **Impact**: The
  epic could satisfy every criterion while producing shallow or poorly-sourced
  research, with no agreed way for the team to notice. **Suggestion**: Add one
  criterion establishing an output-quality bar — for example a worked reference
  topic whose synthesis and citations are reviewed against a stated standard —
  or record explicitly in Assumptions that output quality is evaluated by human
  judgement rather than by acceptance criteria.

### Scope

**Summary**: Work item 0121 is a genuinely epic-sized, coherent capability —
every theme and every slice serves the single purpose of a citation-backed
external-topic research skillset, and the vertical slicing with stated ordering
constraints gives a real orchestration strategy across the four toolchains it
touches. The scope risk is not at the epic level but inside its children: Slice
1 packs roughly a dozen establishing deliverables across skill, agent, template,
config, indexer and visualiser layers into one "walking skeleton", and Slice 5
is simultaneously named as extractable and made load-bearing in the epic's
acceptance criteria, leaving the epic's completion boundary undefined. A
residual, unbounded source category in theme 4 sits inside the epic's scope with
no child and no criterion covering it.

**Strengths**:

- The `epic` kind fits: six themes and five substantial vertical slices
  describing one unified capability, not a grab-bag — every child serves the
  same user-visible outcome of building and browsing an external-topic
  knowledgebase.
- Decomposition is explicitly vertical rather than layer-wise, with each slice
  stated to cut skill → agent → template → artifact → visualiser, and the
  Drafting Notes record the deliberate rejection of a horizontal
  "infrastructure" story — a common epic anti-pattern avoided on purpose.
- Child ordering constraints are made explicit (Slice 1 foundational, Slices 2–4
  reorderable and independently valuable, Slice 5 dependent on the doc type and
  nested indexing from Slices 1–2), so the epic can be sequenced without
  discovering hidden coupling mid-flight.
- The "reusable infrastructure" theme is validated inside the epic rather than
  deferred: two profiles (web in Slice 1, academic in Slice 3) exercise the
  injectable seam, and acceptance criterion 5 asserts it directly — the
  generality has an in-scope consumer pulling it.
- Out-of-scope items are named repeatedly and consistently — the ideation
  skillset, competitor and technical-topic research skills, codebase/internal
  sources, Crossref and Semantic Scholar, and the dedicated citation pass — and
  each deferral is designed as an additive drop-in, which keeps the epic
  boundary stable rather than merely postponed.
- The one overlapping neighbour (`0056`) is scoped away with a stated reason
  (0056 lists external research as a non-goal), so the epic does not depend on a
  parallel thread completing simultaneously.

**Findings**:

- **major / medium** — *Slice 5 is flagged for extraction yet is load-bearing in
  the epic's acceptance criteria* — Location: Requirements: Initial stories —
  Slice 5. Epic 0121 describes Slice 5 — the visualiser set-level detail page
  and sub-document navigation — as a "Candidate for extraction as its own work
  item", yet acceptance criterion 10 requires exactly that deliverable ("its
  detail page links to the brief, each topic, and the synthesis, defaulting to
  the `manifest.primary` sub-document"). Slice 5 is also the only child carrying
  the epic's sole unresolved Open Question (whether a new set endpoint or an
  augmented `/api/docs/{path}` response serves a set's siblings) and the only
  one requiring a Claude Design prototyping step before implementation.
  **Impact**: The epic's completion boundary is undefined — extract Slice 5 and
  the epic can never satisfy its own acceptance criteria; retain it and epic
  closure is gated on an undecided API design and a prototyping activity that
  are independent of the research engine delivering the epic's core value.
  **Suggestion**: Resolve this now rather than at Slice 5 planning: either
  extract Slice 5 into a separate work item and rewrite acceptance criterion 10
  to require only library-entry browsability (which Slice 1 already delivers),
  or keep it in scope and delete the extraction note so the boundary is
  unambiguous.

- **major / medium** — *Slice 1 "walking skeleton" bundles two separately
  shippable increments across four toolchains* — Location: Requirements: Initial
  stories — Slice 1. Slice 1 of epic 0121 is labelled a walking skeleton but
  enumerates roughly a dozen distinct establishing deliverables: a new
  `paths.research_topics` config key plus `docs.rs` wiring, four templates
  resolving through the 3-tier override, per-skill `instructions.md`/`context.md`
  wiring, a `manifest.md` aggregate root, the generic `researcher` agent with
  WebFetch of arbitrary URLs, a web source profile, the `research-topic` skill
  with two subcommands on the dispatch pattern, an orchestrator prompt carrying
  the effort-scaling rubric, hardcoded depth/breadth defaults, full visualiser
  doc-type registration (Rust enum, TS union, glyph, colour tokens, framed
  background, Discover-phase placement, and visual-regression baselines across
  sizes × themes on both darwin and linux), and nested-manifest indexer
  integration. **Impact**: As a single child this is plausibly multi-week and
  fuses two increments that could be delivered, demoed and rolled back
  independently — the research engine that writes artifacts to
  `meta/research/topics/<slug>/`, and the visualiser registration plus indexing
  that makes them browsable — so a stall in either half blocks the epic's
  foundational slice entirely. **Suggestion**: Split Slice 1 into 1a (skill +
  agent + web profile + templates + config key, demoable as artifacts on disk)
  and 1b (umbrella doc-type registration, nested-manifest indexing, VR
  baselines), sequencing 1b immediately after 1a so the end-to-end vertical demo
  still lands early.

- **minor / high** — *Theme 4 keeps an unbounded source category in scope that
  no slice or criterion covers* — Location: Requirements: High-level goals and
  themes — item 4. Requirements theme 4 of epic 0121 places "web and academic
  (and other sensible external) sources" inside the epic's scope, but the slice
  preamble excludes anything beyond web and academic from all five slices, no
  acceptance criterion references additional source families, and Technical
  Notes further defer Crossref and Semantic Scholar as drop-in profiles.
  **Impact**: The epic carries a residual with no natural stopping point ("other
  sensible external sources") that no child delivers and no criterion verifies,
  so the in/out boundary cannot be stated cleanly and epic closure can be
  contested by anyone reading the themes rather than the slices.
  **Suggestion**: Narrow theme 4 to "web and academic sources, realised through
  injectable profiles with credibility tiers" and move additional source
  families to the existing out-of-scope list under Dependencies, alongside the
  future consumer skills.

- **minor / medium** — *Slices 2 and 3 each pair a core behaviour with a
  separately shippable enhancement* — Location: Requirements: Initial stories —
  Slices 2 and 3. Two children of epic 0121 bundle capabilities that could be
  delivered independently. Slice 2 pairs idempotent multi-round accretion
  (gap-detection re-invocation, immutable topic files, append-only round log,
  wholesale synthesis rewrite, manifest counter sync) with the user-facing
  `expand` subcommand for adding new topics — the accretion machinery is a
  prerequisite for `expand`, but not the reverse. Slice 3 pairs two new academic
  providers (OpenAlex + arXiv, a `research.contact_email` config, Atom-XML
  parsing, rate-limit-aware fan-out) with inline reputation-tier citation
  tagging, which is a cross-cutting concern applying equally to the web sources
  already delivered in Slice 1. **Impact**: Each pairing makes the slice larger
  than its minimum shippable increment and couples a deferrable enhancement to a
  core behaviour, blurring what "done" means for the slice and delaying the
  demoable accretion loop behind a second subcommand's design.
  **Suggestion**: Consider sequencing `expand` as its own slice after the
  accretion loop, and landing reputation-tier tagging against web sources before
  the academic providers — that would de-risk the synthesis carry-forward
  behaviour independently of the arXiv/OpenAlex integration.

---

## Re-Review (Pass 2) — 2026-09-06

**Verdict:** REVISE

Lenses re-run: clarity, completeness, scope (all three had findings in pass 1).

### Previously Identified Issues

**Resolved (12 of 16):**

- 🟡 **Clarity + Completeness**: The artifact contract is named as a core deliverable but never specified — Resolved. The new Artifact contract subsection pins four document shapes at field-name level with a closed `research_kind` set; all three lenses cite it as a strength.
- 🟡 **Clarity**: Agent count governed by two uncoordinated controls — Resolved. `breadth` is now a hard ceiling with the rubric operating strictly beneath it, stated in a Technical Note and a dedicated criterion.
- 🟡 **Clarity**: Source selection stated at three different granularities — Resolved. The brief declares `source_profiles`, the orchestrator assigns one per agent, rounds may be mixed.
- 🟡 **Clarity**: "The synthesiser" is an actor that appears nowhere else — Resolved. The `conduct` orchestrator writes `synthesis.md` inline; the component inventory is stated explicitly.
- 🟡 **Scope**: Slice 5 flagged for extraction yet load-bearing — Resolved. Extraction note removed, Slice 5 kept in scope; the scope lens now raises it only as a documented descope *option*.
- 🔵 **Scope**: Theme 4 keeps an unbounded source category — Resolved. Narrowed to web and academic; other families moved to the Dependencies out-of-scope list.
- 🔵 **Clarity**: One source-quality concept named four different ways — Resolved. "Reputation tier" nominated as the single term and scoped to reputation-only; cited as a strength.
- 🔵 **Clarity**: `manifest.primary` read but never written — Resolved. `brief` writes it, `conduct` flips it, and it is in Slice 2's sync list.
- 🔵 **Clarity**: "The corpus" undefined and conflicting with the round log — Resolved. Two criteria now name the artefacts and exempt `brief.md`'s round log.
- 🔵 **Clarity**: Unbounded qualifiers (breadth default, "any visualiser grouping") — Resolved. `breadth: 8` stated in four places; Slice 2 states no visualiser work is required.
- 🔵 **Completeness**: Context explains provenance but not the motivating problem — Resolved. New opening paragraph; cited as a strength.
- 🔵 **Clarity**: Abbreviations used before definition — Resolved. VR, the three override tiers, INVEST, the polite pool and Claude Design are all glossed on first use; cited as a strength.
- 🔵 **Completeness**: Acceptance criteria not attributed to slices — Resolved as written, but see "New Issues" — the tagging exposed three criterion/slice-body contradictions.
- 🔵 **Clarity**: Claude Design undefined, prototype status ambiguous — Resolved. Glossed, and prototyping stated as in-slice work.
- 🔵 **Clarity**: Resolved questions restated twice under a misleading heading — Resolved, though the replacement cross-reference now points at the wrong bullet.
- 🔵 **Completeness**: No criterion addresses research output quality — Resolved. Human-judged worked reference topic added as a criterion, with a matching Assumption.

**Partially resolved:**

- 🔵 **Completeness**: Frontmatter status and priority provisional — Partially resolved. `priority` raised to `high` and the provisional caveats removed; `status: draft` deliberately untouched because this command must not modify `status`.
- 🔵 **Completeness**: Slices not linked to child work items — Partially resolved. The preamble now states each slice becomes a child before implementation, but the clarity lens notes the sentence names no actor, no frontmatter key, and no target document.

**Still present (accepted):**

- 🟡 **Scope**: Slice 1 bundles two separately shippable increments — Still present, by decision. The scope lens re-raises it at major severity, acknowledges the decision is recorded in Drafting Notes, and asks that the rationale move into the slice text so the child work item inherits it rather than the objection.
- 🔵 **Scope**: Slices pair core behaviour with separable enhancement — Still present, by the same decision. The lens now points at Slice 4 (cost knob + recursion engine) rather than Slices 2–3, arguing the cheap cost guard cannot ship early because it is bundled with the epic's most deferrable capability.

### New Issues Introduced

Five of the seven below were introduced by the pass-1 edits themselves — the artifact contract and the slice tagging each created contradictions with existing slice text.

- 🟡 **Clarity + Scope**: Reputation-tier tagging is mandatory in the contract but deferred to Slice 3. The contract states "Every entry under a topic file's Sources carries an inline reputation tier" while Slice 1's web profile only captures URLs. Because the same section declares topic files immutable, Slices 1–2 would write a permanently non-conforming corpus. Flagged independently by both lenses.
- 🟡 **Clarity**: Slice 1 acceptance criteria claim work the slice bodies assign to Slice 2 — `expand` dispatch, next-round proposal, and manifest sync are all tagged *(Slice 1)* but delivered by Slice 2 per the slice text. Introduced by the slice-tagging edit.
- 🟡 **Clarity**: "Topic" denotes both the research subject and the per-agent sub-question. Pre-existing in the prose, but the contract's binding `topic` and `topic_count` field names sharpen it into an implementation ambiguity.
- 🟡 **Completeness**: Slice 4's depth-recursion engine has no acceptance criterion — the sole Slice 4 criterion covers only flag/config resolution, so the slice's headline deliverable has no definition of done. Surfaced by the slice tagging.
- 🔵 **Completeness**: Slice 3's `research.contact_email` polite-pool config and rate-limit-aware fan-out have no criteria.
- 🔵 **Completeness**: Slice 5's in-slice design-prototype work has no criterion — introduced by the edit that made prototyping explicitly in-scope.
- 🔵 **Clarity**: The Open Questions cross-reference to "the final bullet of Drafting Notes" now points at the appended Review 1 bullet rather than the refinement-session bullet. Introduced by the pass-1 edits.
- 🔵 **Clarity**: `research_status: complete` has no stated transition trigger in a model whose premise is that a set is always expandable.
- 🔵 **Clarity**: Work item `0056` is described as `meta/research/codebase/` in Context but `meta/research/` in Dependencies and References. Pre-existing; not caught in pass 1.
- 🔵 **Scope**: The slice preamble's claim that each slice cuts through all layers is now inaccurate — Slice 2 states no visualiser work is required and Slice 5 is visualiser-only. Introduced by the pass-1 edit to Slice 2.
- 🔵 **Clarity**: "Slices 2–4 are reorderable" conflicts with Slice 4 wiring flags into `expand`, which Slice 2 delivers.
- 🔵 **Clarity**: The rubric's "10+" band is unreachable in Slices 1–3 because `--breadth` does not exist until Slice 4, and "the first cost guard" reads as either first-delivered or primary.
- 🔵 **Completeness**: No requirement or criterion covers user-facing documentation for the new skill beyond `configure help`.

### Assessment

The epic is substantially better than at pass 1: the two structural gaps that mattered most — an unspecified artifact contract and an undefined epic boundary — are closed, and all three lenses now cite the contract, the terminology discipline and the named actors as strengths. The verdict stays REVISE because the pass-1 edits introduced a second-order problem: pinning the contract and tagging the criteria created contradictions with slice text that was written before either existed.

The remaining work is narrower and mostly mechanical. Two decisions are genuinely open — whether the web profile emits reputation tiers from Slice 1 (making the contract honest) or pre-Slice-3 topic files are a documented exception, and which slice owns `expand`, next-round proposal and manifest sync. The rest is cross-reference repair, three missing acceptance criteria, and a terminology sweep separating the research subject from the per-agent sub-question. Slice 1's size and Slice 4's pairing remain open by deliberate decision, not oversight.

---

## Re-Review (Pass 3) — 2026-09-07

**Verdict:** REVISE

Lenses re-run: clarity, completeness, scope. The scope reviewer was told that Slice 1's size and Slice 4's pairing are informed decisions with stated rationale, so it could judge the rationale rather than re-raise the sizing cold.

### Previously Identified Issues

**Resolved:**

- 🟡 **Clarity + Scope**: Reputation-tier tagging mandatory in contract but deferred to Slice 3 — Resolved. The clarity lens confirms the story is now "internally consistent end to end" across the contract note, Slice 1's web profile, Slice 3's "purely additive" framing and the Slice 1 criterion.
- 🟡 **Clarity**: Slice 1 criteria claiming Slice 2 work — Resolved for `expand` and next-round proposal. Manifest-sync ownership is still double-claimed (see below).
- 🟡 **Scope**: Slice 1 bundles two separately shippable increments — Resolved as a sizing objection. The stated rationale was accepted and the major dropped; what remains is a minor observation that the *specific* argument used ("artifacts on disk with no library entry are not demoable") is contradicted by Slices 2 and 3 shipping with no visualiser work.
- 🟡 **Completeness**: Slice 4's recursion engine has no criterion — Resolved. A criterion now covers no-recursion at `depth: 1` and halving per descent.
- 🔵 **Completeness**: Slice 3's polite-pool config and Slice 5's prototype have no criteria — Resolved. Both added.
- 🔵 **Clarity**: Drafting Notes cross-reference pointing at the wrong bullet — Resolved. Now references by content.
- 🔵 **Clarity**: `0056` described two ways — Resolved. Unified to `meta/research/codebase/` in all three places, with its `done` status noted.
- 🔵 **Clarity**: "Slices 2–4 are reorderable" conflicting with dependencies — Resolved. Ordering constraints now state what holds, including Slice 4 following Slice 2.
- 🔵 **Clarity**: Rubric's "10+" band unreachable — Resolved as a gap, though the sentence that fixed it introduced two new ambiguities ("strictly beneath" versus the criterion's "never exceed", and a dangling "which").
- 🔵 **Completeness**: No documentation requirement — Resolved. Folded into the all-slices criterion.
- 🔵 **Clarity**: "Topic" denoting two things — Partially resolved. The new Terminology section fixes the definition and is cited as a strength by all three lenses, but two later statements still use "topic" in the subject sense the section forbids.

**Still present (accepted):**

- 🔵 **Scope**: Slice 4 pairs the cost knob with the recursion engine — Still present by decision. The stated rationale was accepted; the lens now only observes that the in-slice fallback would ship a documented `depth` key with no behaviour.

### New Issues Introduced

As in pass 2, most of these were introduced by the pass-2 edits rather than being pre-existing.

- 🟡 **Clarity + Completeness**: **`breadth` as a per-round cap is arithmetically incompatible with intra-round recursion.** "Agents spawned per round never exceed `breadth`" cannot hold alongside `depth > 1` drilling each topic within the same round with fan-out halving from `breadth`. At `breadth: 8, depth: 2` a round spawns 8 top-level agents plus up to 4 sub-agents each. The two readings differ several-fold in spend on the epic's nominated cost guard. Flagged independently by both lenses; the most consequential finding of this pass.
- 🟡 **Completeness + Clarity + Scope**: **`research_status: complete` is unreachable and has no actor.** The contract forbids `conduct` and `expand` from setting it, `brief` sets `briefed`, no slice or criterion mentions it, and "`update-work-item`-style intent" names nothing in this epic. A declared contract value ships dead while future consumer skills code against it.
- 🟡 **Clarity**: **An ambiguous "it" makes the `research_status` note contradict itself** — "nothing in `conduct` or `expand` may set it" reads as either the `complete` value or `research_status` itself, and the second reading contradicts the same sentence and Slice 2.
- 🟡 **Clarity + Completeness**: **"Topic" used in the subject sense in two places** — the Technical Notes breadth entry ("a simple topic may spawn 1 agent") and the Assumptions quality-gate bullet ("worked reference topic", where the criterion says "subject"). In the breadth note the wrong sense changes what the cost guard means.
- 🟡 **Scope**: **The only output-quality gate sits in the one slice declared freely reorderable.** Slice 1 claims to be "the smallest increment that tests the premise", but the human-review criterion that tests it is tagged Slice 3, which has no ordering constraint and could ship last — so the premise can go unvalidated through Slices 1, 2 and 4, including the token-multiplying recursion engine.
- 🔵 **Clarity + Completeness + Scope**: Slice 5's prerequisite is stated as "Slice 1" in the ordering paragraph and "Slices 1–2" in Slice 5's body; the first-round `primary` flip is claimed by both Slice 1's criterion and Slice 2's body. All three lenses flagged this.
- 🔵 **Clarity + Completeness + Scope**: The slicing invariant is now stated three incompatible ways — Drafting Notes still says every slice cuts through all layers, the preamble names Slices 2 and 5 as the only exceptions, and Slice 4's contents make it an unacknowledged third exception. All three lenses flagged this.
- 🔵 **Completeness**: The artifact contract has no representation for depth-recursion sub-topics — no `level` or `parent_topic` field, and no statement of whether sub-level findings get their own file or fold into the parent. Discovering this at Slice 4 means a late contract change over immutable files.
- 🔵 **Scope**: `source_profiles` has no owning slice — Slice 1's criterion asserts brief conformance to the contract while a Slice 3 criterion says the brief declares the list.
- 🔵 **Completeness**: Three commitments still lack criteria — topic-file and `synthesis.md` contract conformance (asserted only for brief-time artifacts), Slice 3's tier rendering, and the temp-dir-then-rename atomicity rule from the Indexer note.
- 🔵 **Clarity**: Slice 3's "rendering of their tiers" is unclear now that Slice 1 emits tiers through the same shared view — the slice's rendering delta is unstated.
- 🔵 **Clarity**: The `<path>` positional's referent is never stated — `brief.md` or the set directory. Argument handling, gap detection and manifest updates all key off it.
- 🔵 **Scope + Clarity**: Two descope paths exist (Slice 5 whole-slice deferral, Slice 4 in-slice reduction) while the preamble calls Slice 5 "the only" one; the Slice 4 fallback would ship a documented but inert `depth` key.
- 🔵 **Clarity**: Themes 1 and 6 lag the decisions pinned later — theme 1 says "brief/topic/synthesis" where the contract has four documents including the manifest; theme 6 hedges "type(s)" where the doc type is committed as one.
- 🔵 **Clarity**: "Set" does structural work throughout (set lifecycle, set-level page, a set's siblings, Set Contents) but is never defined alongside subject and topic.
- 🔵 **Completeness**: The new generic `researcher` agent is never assigned to the config override surface that theme 5 and Context both claim conformance with.

### Assessment

Not ready, and the reason is now a pattern rather than a list. Three passes in, each round of edits resolves its targets and introduces a comparable number of new inconsistencies — five of this pass's six majors were created by the pass-2 edits, exactly as five of pass 2's seven were created by pass 1.

The mechanism is duplication, not carelessness. Every fact in this work item is stated in two or three places — a slice body, an acceptance criterion, a Technical Note, sometimes the contract table — so a single correction requires finding and updating every copy, and the density (Slice 1 is one ~350-word sentence) makes copies easy to miss. The lenses are consistently catching real contradictions between copies, which means the document's structure, not its content, is generating the findings.

Two genuine decisions remain: what unit `breadth` caps (per level, or per round including sub-levels), and whether `research_status: complete` gets a mechanism in this epic or is declared reserved. The rest is copy reconciliation.

Recommendation: stop patching and do one consolidation pass that removes the duplication — make each fact single-sourced, with slice bodies and criteria referring to the contract rather than restating it — then re-review once. Continuing to patch copy-by-copy will keep producing this result.

---
*Review generated by /accelerator:review-work-item*
