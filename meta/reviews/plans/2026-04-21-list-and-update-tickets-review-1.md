---
date: "2026-04-21T08:57:25Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-21-list-and-update-tickets.md"
review_number: 1
verdict: REVISE
lenses: [ architecture, code-quality, test-coverage, correctness, standards, usability, compatibility, safety ]
review_pass: 1
status: complete
---

## Plan Review: Ticket Listing and Updating Skills (Phase 3)

**Verdict:** REVISE

The plan carries forward Phase 2's structural discipline and correctly
identifies legacy-schema coexistence as a first-class concern; its
TDD framing with 14+22 approach evals gives skill-creator a concrete
specification to iterate against. However, six spec surfaces are
repeatedly flagged across lenses as under-tightened in ways that will
produce inconsistent behaviour or silent data corruption: template-
comment hint extraction, the body `**Status**:` sync, tags array
re-serialisation, natural-language filter/op grammar, parent-cycle
detection, and the title-column source for new-schema tickets. The
Current-State analysis also contains a factual inversion about legacy
schemas that propagates into at least one eval, and several behaviours
asserted in Quality Guidelines (cycle detection, multi-op combined
diffs, confirmation-token tolerance) have no corresponding scenario.

### Cross-Cutting Themes

- **Template-comment hint extraction is load-bearing but unspecified**
  (flagged by: architecture, code-quality, correctness, compatibility,
  usability) — Both skills depend on parsing trailing `# a | b | c`
  comments in `templates/ticket.md` at runtime, but no lens found a
  parsing rule, no fallback is defined for user-overridden templates
  that omit the comments, and the two skills re-derive the rule
  independently so they can drift.
- **Body `**Status**:` sync spec is ambiguous in six ways** (flagged
  by: architecture, code-quality, test-coverage, correctness,
  compatibility, safety) — regex anchoring, multi-match handling,
  code-fence false-positives, title-casing of hyphenated/multi-word
  values, sibling labels (`**Type**:`/`**Priority**:`/`**Author**:`
  appear in the same template but are not synced), and atomicity
  across frontmatter+body edits are all under-specified.
- **Tag array parse → mutate → re-serialise round-trip is a data-loss
  hazard** (flagged by: architecture, test-coverage, correctness,
  usability, compatibility, safety) — quoting rules, flow-vs-block
  style, spacing, empty `tags: []`, absent field, and tags containing
  special characters are all unaddressed, yet the LLM is expected to
  write canonical YAML back to disk.
- **Natural-language filter/op grammar lacks a precedence rule**
  (flagged by: code-quality, test-coverage, correctness, usability)
  — "drafts", "bug", "backend", "review" are all plausibly field
  values, tags, or title searches; the spec's "clearly indicates a
  field" phrasing is not testable and will produce run-to-run drift.
- **Parent-cycle detection is asserted but has no eval** (flagged by:
  code-quality, test-coverage, correctness, safety) — Quality
  Guidelines require it for list-tickets hierarchy mode but no
  scenario exercises it, so the guard could regress silently.
- **Parent field canonicalisation (padding + quoting) unspecified**
  (flagged by: code-quality, test-coverage, correctness) — update
  writes `parent: "0001"` (quoted, padded) but filter matches on
  `0042` (unquoted); a user who types `/update-ticket 0042 parent 1`
  could get `parent: "1"` which then fails to match any hierarchy or
  `under 0001` filter.
- **Multi-op combined edits specified but not evaluated** (flagged by:
  test-coverage, correctness, safety) — Step 3 promises
  `status ready priority high` produces one combined diff, but no
  scenario tests it and the grammar for interleaving structured + tag
  + natural-language ops is ambiguous.
- **Legacy ticket schema coexistence has three correctness gaps**
  (flagged by: correctness, compatibility, usability) — Scenario 14's
  claim that legacy tickets fall back to body-heading title is
  inverted (they have `title:` frontmatter; new-schema tickets do not);
  NL filter shorthands silently exclude the entire 29-ticket legacy
  corpus; and legacy tickets render as walls of `—` in the default
  table.

### Tradeoff Analysis

- **Safety vs. Usability on affirmative confirmation**: Safety lens
  wants case-insensitive exact `y`/`yes` with any other input treated
  as decline (fail-safe). Usability lens wants a re-prompt on
  unrecognised input rather than silent abort (fewer dead-ends).
  Recommend: exact `y`/`yes` accepted; one re-prompt on unrecognised
  input before aborting.
- **Standards consistency vs. least privilege on `list-tickets`
  allowed-tools**: Standards lens notes the `tickets/scripts/*` Bash
  permission is unnecessary for list-tickets (which invokes no Phase 1
  script). Architecture/compatibility lenses prefer structural
  consistency with Phase 2. Recommend: narrow to `config-*` only —
  the "four-file grep" cross-skill convention already verified in 3.3
  does not require allowed-tools to be identical.
- **Rename `ticket_id` warn-vs-block**: Architecture + safety lenses
  split — warn-and-proceed creates a silently inconsistent state;
  hard-block forces users toward a manual `jj mv` + edit workflow.
  Recommend hard-block with a concrete pointer to the rename
  workflow, given the filename is already authoritative.

### Findings

#### Major

- 🟡 **Correctness**: Title column source is unresolved — new-schema tickets have
  no `title:` frontmatter
  **Location**: Subphase 3.1 Scenarios 1, 10, 14
  `templates/ticket.md` has no `title:` frontmatter field; legacy tickets DO.
  Scenario 14's fallback logic is inverted. Without a title-resolution rule, the
  Title column will be empty for every new-schema ticket.

- 🟡 **Correctness / Compatibility**: Body sync covers only `**Status**:` —
  `**Type**:`, `**Priority**:`, `**Author**:` body lines also exist in the
  template
  **Location**: Subphase 3.2 Scenario 16 + Step 4
  `/update-ticket 0042 priority high` will produce `priority: high` in
  frontmatter but leave `**Priority**: Medium` in the body — silent internal
  inconsistency. Either extend body sync to all four label lines or explicitly
  exclude them in "What We're NOT Doing".

- 🟡 **Correctness / Safety / Architecture / Compatibility**: Body `**Status**:`
  sync regex is ambiguous; can silently corrupt body content
  **Location**: Subphase 3.2 Scenario 16 + Step 4
  Multi-match, code-fence matching, and case-mismatch handling are unspecified.
  Title-casing of `waiting-on-legal` (Scenario 18) is ambiguous (
  `Waiting-On-Legal`? `Waiting-on-legal`?) and `review-adr`'s closed-vocabulary
  precedent does not resolve it.

- 🟡 **Safety / Correctness / Test-coverage / Architecture / Compatibility**: Tag
  re-serialisation has no format-fidelity guarantee
  **Location**: Subphase 3.2 Scenarios 9–10, Quality Guidelines
  Quoted elements, block-style arrays, special characters, `tags: []`, absent
  field, and spacing conventions are all unaddressed. A single `add tag` op on a
  block-style ticket could flatten the list to flow style; a tag with a space
  could be unquoted into invalid YAML.

- 🟡 **Correctness**: Tag-array parsing tested on a single happy shape only
  **Location**: Subphase 3.1 Scenario 7
  Quoted values, block-list tags, absent field, and case-sensitivity are not
  covered. Real tickets written by users may use any of these forms.

- 🟡 **Compatibility**: Legacy title-fallback claim contradicts actual legacy
  schema
  **Location**: Subphase 3.1 Scenario 14
  All 29 legacy tickets carry `title:` frontmatter. The body-heading fallback
  path is both unnecessary for current data and wrong about the
  `# ADR Ticket: ...` heading shape if ever triggered.

- 🟡 **Compatibility / Test-coverage**: Natural-language filter shorthands
  silently exclude the entire legacy corpus
  **Location**: Subphase 3.1 Scenarios 3, 4 + Subphase 3.3
  `only drafts`/`epics` map to template defaults; all 29 legacy tickets have
  `status: todo|done` and `type: adr-creation-task`. No shorthand documented for
  legacy values; Subphase 3.3 accepts the zero-match as correct.

- 🟡 **Correctness / Code-quality / Usability**: Natural-language filter grammar
  lacks a precedence rule
  **Location**: Subphase 3.1 Step 1; Scenarios 3–6, 14
  "Clearly indicates a field" is not testable. `/list-tickets backend`,
  `/list-tickets review`, `/list-tickets draft` are all ambiguous under the
  current rules and will drift in interpretation.

- 🟡 **Code-quality**: Template hint extraction mechanism unspecified — both
  skills share the same fragile contract
  **Location**: Subphase 3.1 Step 1; Subphase 3.2 Scenario 17 + Quality
  Guidelines
  Parsing rule (separator, comment marker, fallback) is defined nowhere. Both
  skills will re-derive it and drift.

- 🟡 **Architecture**: Implicit contract between template trailing-comment format
  and runtime hint parser
  **Location**: Subphase 3.1 Step 1; Subphase 3.2 Scenario 17
  User-overridable template × informal textual convention × no codified
  contract = silent hint degradation when users customise.

- 🟡 **Architecture / Code-quality**: Frontmatter parsing logic duplicated across
  bash scripts and two LLM prompts
  **Location**: Subphase 3.1 Step 2; Subphase 3.2 Step 2
  `ticket-read-field.sh` is single-field; the LLM must reimplement multi-field
  parsing in two places. Three parsers for one format will drift on edge cases.

- 🟡 **Code-quality**: Edit-application step is vague about how the frontmatter
  block is rewritten
  **Location**: Subphase 3.2 Step 5
  Quoting preservation, trailing-comment preservation, and array round-tripping
  are not pinned. Edit tool's unique-match requirement means edits can fail
  silently on legacy tickets whose formatting differs.

- 🟡 **Code-quality**: Inconsistent error-handling contract for malformed
  frontmatter across sibling skills
  **Location**: Subphase 3.1 Scenario 11 vs Subphase 3.2 Scenario 22
  list-tickets warns-and-continues; update-ticket aborts; error messages differ
  from the canonical `ticket-read-field.sh` output.

- 🟡 **Test-coverage / Code-quality / Correctness / Safety**: Parent-cycle
  detection required but has no eval
  **Location**: Subphase 3.1 Quality Guidelines
  Guideline mandates cycle-safe hierarchy rendering; no scenario exercises it.

- 🟡 **Test-coverage / Correctness / Safety**: Multi-op combined edits have no
  eval scenario
  **Location**: Subphase 3.2 Step 3 + Manual Verification checklist
  An authored SKILL.md that processes only the first op, or issues one
  confirmation per op, would pass every listed eval.

- 🟡 **Test-coverage**: Body-status sync scenario under-specifies matching rules
  **Location**: Subphase 3.2 Scenario 16
  Exact-case happy path only; no coverage of different-case bodies, drift, or
  title-case rule enforcement.

- 🟡 **Test-coverage**: Tag add/remove evals miss key edge cases the bash layer
  cannot catch
  **Location**: Subphase 3.2 Scenarios 9, 10
  Tags absent, `tags: []`, block-list form, values with commas/quotes, removing
  the last tag — all unaddressed in a surface the bash regression suite does not
  cover.

- 🟡 **Test-coverage**: Read-only-field warning path lacks a decline scenario
  **Location**: Subphase 3.2 Scenario 21
  User typing `n` at the read-only warning is untested; a double-prompt UX bug
  would slip through.

- 🟡 **Correctness / Test-coverage**: Parent-value canonicalisation across
  padded/unpadded forms is untested
  **Location**: Subphase 3.1 Scenario 8; Subphase 3.2 Scenario 11
  `parent: 42` vs `parent: "0042"` matching is unspecified; YAML would interpret
  `parent: 0001` as integer `1`.

- 🟡 **Correctness / Usability**: Multi-op argument grammar is ambiguous for
  mixed structured/tag/NL ops
  **Location**: Subphase 3.2 Step 3
  No separators or quoting rules; `status priority`, `priority high add tag x`,
  multi-word values all indeterminate.

- 🟡 **Safety / Architecture**: `ticket_id` warning understates cascading
  breakage
  **Location**: Subphase 3.2 Scenario 21
  The filename is authoritative, so an edit-without-rename produces a silently
  inconsistent ticket the user will discover days later.

- 🟡 **Usability**: Natural-language filter parsing has ambiguous cases that will
  produce wrong filters silently
  **Location**: Subphase 3.1 Scenarios 3, 5, 14
  Disambiguation rule is not deterministic; same query returns different results
  across invocations.

- 🟡 **Usability**: Bare `/update-ticket` does not teach the user what ops are
  valid
  **Location**: Subphase 3.2 Scenarios 1, 6
  No op-language cheatsheet; users guess at syntax or re-read SKILL.md.

- 🟡 **Usability**: Field-name-only op collides with natural-language arguments
  **Location**: Subphase 3.2 Scenarios 12, 17
  `/update-ticket 0042 status` vs `/update-ticket 0042 status maybe` vs
  `/update-ticket 0042 mark as done` — no trigger condition for
  hint-elicitation.

- 🟡 **Test-coverage**: Tag-array parsing tested only against `[backend, api]`
  happy shape
  **Location**: Subphase 3.1 Scenario 7
  See Correctness finding above.

- 🟡 **Architecture**: Implicit contract — template trailing-comment format and
  runtime parser
  **Location**: Both skills
  See cross-cutting themes.

#### Minor

- 🔵 **Architecture**: Body status-line sync imports closed-vocabulary pattern
  into open-vocabulary context (Subphase 3.2 Step 4)
- 🔵 **Architecture**: `ticket_id` warn-without-rename creates persistent
  filename/frontmatter divergence (Subphase 3.2 Scenario 21)
- 🔵 **Architecture**: Legacy-ticket filter semantics silently exclude rather
  than surface asymmetry (Subphase 3.1 Step 3, Scenario 10)
- 🔵 **Architecture**: Integration verification exercises form but not
  cross-skill data-flow invariants (Subphase 3.3)
- 🔵 **Code-quality**: Cycle-detection requirement in Quality Guidelines has no
  eval (Subphase 3.1)
- 🔵 **Code-quality**: Inconsistent quoting convention for `parent` field (
  Subphase 3.2 Scenario 11 vs Subphase 3.1 Scenario 8)
- 🔵 **Code-quality**: Body status-sync regex is narrow; title-casing rule for
  multi-word kebab-case values undefined (Scenario 16, Step 4)
- 🔵 **Code-quality**: Hierarchy mode behaviour on legacy tickets missing
  `parent` is unstated (Scenarios 10, 13)
- 🔵 **Code-quality / Test-coverage**:
  `grep -iE "transition|draft.*ready|invalid.*status"` automated check is
  brittle/theatre (Subphase 3.2 Success Criteria)
- 🔵 **Code-quality**: Mixed interpretation-echoing contract across Scenarios
  7/8/9/10/11/12/18 (Subphase 3.2 Step 3)
- 🔵 **Code-quality**: "em-dash or blank" for missing fields is under-specified
  and self-inconsistent (Scenario 10 vs Step 4)
- 🔵 **Test-coverage**: Natural-language filter parser has no negative-case
  eval (Scenarios 3–6)
- 🔵 **Test-coverage**: Confirmation-token acceptance is under-asserted (Scenario
  13)
- 🔵 **Test-coverage**: Integration verification's only end-to-end check is a
  single manual happy-path (Subphase 3.3)
- 🔵 **Test-coverage**: Hierarchy + legacy-missing-parent interaction is
  untested (Scenarios 10, 13)
- 🔵 **Correctness / Code-quality**: Title-casing of hyphenated status values
  undefined (Scenario 16, Step 4)
- 🔵 **Correctness**: Template-comment parsing for hints brittle and
  unspecified (Scenario 17)
- 🔵 **Correctness**: Path-like detection heuristic may misclassify bare
  filenames (Subphase 3.2 Step 1)
- 🔵 **Correctness**: Filter-term disambiguation rules are informal (Scenarios
  3–6)
- 🔵 **Correctness**: Re-entry flow after "ask what to change" prompt is
  unspecified (Scenario 6)
- 🔵 **Standards**: Template section spec omits `## Ticket Template` heading and
  framing paragraph used by both Phase 2 exemplars
- 🔵 **Standards**: Agent fallback spec does not require canonical
  `If no "Agent Names" section appears above, use these defaults:` phrasing
- 🔵 **Standards**: Plan does not specify the H1 skill heading that Phase 2
  exemplars carry
- 🔵 **Standards**: `list-tickets` `allowed-tools` includes `tickets/scripts/*`
  but the skill invokes no Phase 1 script
- 🔵 **Standards**: Plan does not explicitly require plain multi-line description
  scalars with 2-space continuation indent
- 🔵 **Standards**: `grep -r "disable-model-invocation: true"` expected-count
  assertion in Subphase 3.3 is fragile
- 🔵 **Usability**: Confirmation prompt response tolerance unspecified (Scenario
  13)
- 🔵 **Usability**: Hierarchy vs `under 0042` distinction subtle;
  `/list-tickets hierarchy under 0042` composition undocumented (Scenarios 8,
  13)
- 🔵 **Usability**: Tag ops without `add`/`remove` keyword are ambiguous (
  Scenario 9; Step 3)
- 🔵 **Usability**: Malformed-frontmatter abort echoes glob instead of resolved
  path (Scenario 22)
- 🔵 **Usability**: argument-hint values are terse; do not signal
  natural-language breadth (both skills)
- 🔵 **Usability**: Legacy tickets render as wall of em-dashes; default table low
  signal-to-noise (Scenario 10, Step 4)
- 🔵 **Usability**: Ambiguous number match aborts without offering numeric
  selection (Subphase 3.2 Scenario 5) — diverges from `review-adr` precedent
- 🔵 **Usability**: Fallback behaviour when template has no comment annotations
  unspecified (Scenario 17)
- 🔵 **Compatibility**: Array re-serialisation contract with
  `ticket-read-field.sh` not formalised (Scenarios 9, 10)
- 🔵 **Compatibility**: Conflation of "empty directory" and "directory does not
  exist" in Scenario 2
- 🔵 **Compatibility**: Plugin scanning of `skills/tickets/` includes non-skill
  `scripts/` subdirectory (Current State Analysis)
- 🔵 **Compatibility**: Template-comment parser coupled to shipping template's
  exact formatting (Step 3, Quality Guidelines)
- 🔵 **Safety**: No atomicity guarantee for multi-line edits (Subphase 3.2 Step
  5) — frontmatter-and-body partial write risk
- 🔵 **Safety**: Affirmative-response matching under-specified (Subphase 3.2 Step
  4)
- 🔵 **Safety**: Combined multi-op diff has no per-op opt-out (Subphase 3.2 Step
  3)
- 🔵 **Safety**: Cycle-detection guarantee for hierarchy rendering asserted but
  not evaluated (Subphase 3.1)

#### Suggestions

- 🔵 **Architecture**: Tag array round-trip owned by no single component;
  consider narrow `ticket-update-tags.sh` helper
- 🔵 **Compatibility**: Read-only warning for `date` field mentions compatibility
  concerns that don't apply; split from `ticket_id`

### Strengths

- ✅ TDD framing is explicit and repeated in three places; next maintainer cannot
  miss the sequence.
- ✅ Phase 2 structural conventions faithfully mirrored: inline `allowed-tools`,
  `accelerator:` agent prefix, configuration preamble ordering, path-injection
  bold-label convention, instructions-injection at end of file.
- ✅ "What We're NOT Doing" enumerates six concrete exclusions, drastically
  reducing scope-creep surface.
- ✅ Legacy-schema coexistence is first-class (Scenarios 10, 19) rather than an
  afterthought.
- ✅ Filename `NNNN` prefix declared authoritative ticket identifier,
  establishing a single source of truth.
- ✅ Diff preview + explicit `y/n` confirmation always gates writes; Scenario 14
  is byte-for-byte verifiable.
- ✅ No-op detection (Scenario 20) and read-only-field warnings (Scenario 21)
  guard foot-guns without blocking legitimate use.
- ✅ Echo-back of interpreted filter above the table (Step 4) and
  `Interpreted as: <field> → <value>` (Scenario 12) are strong self-correcting
  affordances.
- ✅ Template defaults treated as hints rather than a closed set; custom values
  allowed (Scenarios 17, 18).
- ✅ Ambiguous-number glob aborts safely (Scenario 5) rather than silently
  picking one.
- ✅ Malformed frontmatter aborts cleanly with no auto-repair (Scenario 22),
  preventing cascading corruption.
- ✅ Pattern sources cited by file AND line range (`review-adr/SKILL.md` lines
  32–70 and 180–214).
- ✅ Cycle detection called out in Quality Guidelines (though untested — see
  findings).

### Recommended Changes

Ordered by impact. Addressing #1 through #7 would move the plan to APPROVE.

1. **Fix the title-column source** (addresses: Correctness "Title column
   unresolved", Compatibility "Legacy title-fallback contradiction")
   Update Current-State Analysis to record that legacy tickets DO have `title:`
   frontmatter and new-schema tickets DO NOT. Rewrite Scenario 14 and Scenario 1
   rendering so the Title column reads `title` frontmatter for both schemas,
   with a body H1 fallback only for tickets missing both. Add an eval covering a
   new-schema ticket's title actually appearing.

2. **Pin the body-label-sync spec** (addresses: 6 findings across
   correctness/safety/architecture/compatibility/code-quality/test-coverage)
   In Subphase 3.2 Step 4 and Scenario 16, specify: (a) match only the first
   occurrence of `^**Status**: ` outside fenced code blocks, (b) require exact
   `**Status**:` label casing, (c) define the title-case rule explicitly for
   hyphenated/multi-word values (recommend "capitalise each hyphen-separated
   segment"), (d) decide and document whether `**Type**:`/`**Priority**:`/
   `**Author**:` are synced alongside `**Status**:` (recommend: yes, for
   consistency with the template) OR explicitly scope to Status only in "What
   We're NOT Doing", (e) use a single Edit call that rewrites frontmatter + body
   together (or specify ordering + partial-write recovery).
   Add scenarios: (16a) exact-case happy, (16b) different-case body, (16c) body
   value drift from frontmatter, (16d) `**Status**:` inside a code fence, (16e)
   multiple body matches.

3. **Constrain and test tag array serialisation** (addresses: 6 findings across
   safety/correctness/test-coverage/architecture/usability/compatibility)
   Restrict supported tags format to flow-style inline arrays: `[a, b, c]` with
   single-space-after-comma, no quotes unless value contains `,` or `:`. In Step
   3, specify behaviour for: tags absent, `tags: []`, last-tag removal (result
   must be `tags: []`), block-style (abort with clear error), quoted elements (
   preserve), values containing special characters (reject or quote
   deterministically). Add scenarios covering each case. Consider a narrow
   `ticket-update-tags.sh` helper to own the canonical serialisation.

4. **Add a filter/op grammar precedence rule** (addresses: 4 findings across
   code-quality/test-coverage/correctness/usability)
   Add to Subphase 3.1 Step 1 a numbered precedence list: (1) presentation
   keywords (`hierarchy`, `as a tree`), (2) structured shapes with explicit
   keywords (`tagged X`, `under X`, `children of X`, `<field>: <value>`), (3)
   multi-field shorthand (`bugs in review`) only when each token is a value
   present on any ticket (not just the template defaults), (4) single-token
   shorthand matching any present value, (5) otherwise free-text title search.
   If a token matches multiple fields, ask for disambiguation. Add adversarial
   scenarios: `/list-tickets backend`, `/list-tickets review`. Mirror the same
   precedence for `/update-ticket` op parsing in Subphase 3.2 Step 3 and add a
   dedicated rule for `tag <value>` without `add`/`remove` (recommend: prompt
   for disambiguation, never silently overwrite).

5. **Specify template-comment parsing and fallback** (addresses: 5 findings
   across architecture/code-quality/correctness/compatibility/usability)
   Either (a) introduce `ticket-template-field-hints.sh ticket <field>` helper
   that encapsulates the rule in one tested place, or (b) codify the parsing
   rule in Quality Guidelines: "values are the `|`-separated tokens following
   the first `#` on the frontmatter line, trimmed". Either way, specify graceful
   degradation when the template has no trailing comment: skip the hints line,
   proceed to prompt. Add an eval covering a user-overridden template without
   comments.

6. **Parent canonicalisation** (addresses: Correctness, Code-quality,
   Test-coverage on parent handling)
   Specify: parent values are always written as zero-padded 4-digit quoted
   strings (`parent: "0001"`). Filter/hierarchy matching normalises both sides
   before comparison. Add an eval: `/update-ticket 0042 parent 1` writes
   `parent: "0001"`; `/list-tickets under 1` matches it.

7. **Cover parent-cycle detection and multi-op flows with evals** (addresses:
   parent-cycle theme; multi-op theme)
   Add Scenario 15 to Subphase 3.1: `/list-tickets hierarchy` with A.parent=B
   and B.parent=A → both render at top level with a `(cycle)` marker; skill
   terminates in bounded time.
   Add a scenario to Subphase 3.2:
   `/update-ticket 0042 status ready priority high` → single combined diff,
   single confirmation prompt, single Edit operation applying both.
   Add a scenario: `/update-ticket 0042 ticket_id 9999` with user declining at
   the read-only warning → no further prompt, no write.

8. **Standards touchups** (addresses: 6 standards findings)
   In both subphase specs, explicitly require: (a) H1 skill heading after
   frontmatter (`# List Tickets`, `# Update Ticket`), (b) canonical
   `## Ticket Template` heading + framing paragraph around the template
   injection, (c) canonical agent-fallback phrasing
   `If no "Agent Names" section appears above, use these defaults:`, (d) plain
   multi-line description scalars with 2-space continuation indent (no `>` block
   scalars). In Subphase 3.3, replace the recursive-grep-with-count assertion
   with four explicit per-file grep checks. Narrow `list-tickets`
   `allowed-tools` to `config-*` only, or document the wider scope as an
   intentional consistency choice.

9. **Decide on `ticket_id` warn-vs-block** (addresses: Architecture, Safety on
   ticket_id)
   Recommend hard-block with a pointer to the manual `jj mv` + edit workflow,
   given the filename is authoritative. If keeping the warn-and-proceed stance,
   tighten the warning message to be concrete about which listings and lookups
   will use the filename and effectively ignore the new frontmatter value.

10. **Legacy ticket usability** (addresses: Compatibility "NL shorthands exclude
    legacy", Usability "em-dash wall", Correctness "empty vs missing dir")
    Add a scenario for `/list-tickets only todo` or `/list-tickets status todo`
    returning legacy tickets; document in Quality Guidelines that structured
    `<field> <value>` matches any present value, not just template defaults.
    Consider suppressing columns that are universally empty in the current
    result set. Split Scenario 2 into "empty dir" vs "missing dir" with distinct
    messages (the latter points to `init` / `paths.tickets` config).

11. **Confirmation-token and error-message polish** (addresses: Safety/Usability
    affirmative-matching; Usability error-echo-glob; Usability ambiguous-number
    numeric selection)
    Specify case-insensitive exact `y`/`yes` accepted; unrecognised input
    re-prompts once before aborting. Echo resolved paths (not globs) in error
    messages. For Scenario 5 (ambiguous number), list matches as numbered
    options and accept number-or-path selection (matches `review-adr`
    precedent).

12. **Harmonise malformed-frontmatter messages** (addresses: Code-quality
    inconsistency finding)
    State in both skills' Quality Guidelines that malformed-frontmatter messages
    quote `ticket-read-field.sh` output verbatim (append suffix guidance if
    needed; do not rewrite the core message).

13. **Consider consolidating frontmatter parsing** (addresses: Architecture
    duplication theme)
    Introduce `ticket-read-frontmatter.sh` that emits all fields preserving key
    order (JSON or deterministic key=value stream). Both new skills consume it
    rather than re-implementing parsing in prompt.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan extends the existing ticket-management architecture
consistently with Phase 2, reusing the `review-adr` frontmatter-edit pattern and
the shared config/template infrastructure. Boundaries are clear — `list-tickets`
is strictly read-only, `update-ticket` requires explicit confirmation before any
write, and both correctly treat the filename as the authoritative ticket
identifier. The principal architectural risks are (1) an implicit contract
between the ticket template's trailing-comment formatting and runtime hint
parsing, (2) duplication of YAML frontmatter parsing logic between the existing
bash helpers and the LLM prompts in both new skills, and (3) the body
`**Status**:` sync pattern being borrowed from a closed-vocabulary ADR context
into an open-vocabulary ticket context without reconciling the title-casing
assumption.

**Strengths**:

- Consistent boundaries with Phase 2: identical allowed-tools scope, same config
  preamble, same agent fallback convention
- Filename prefix declared authoritative ticket number over frontmatter
  ticket_id
- Legacy ticket schema coexistence explicitly addressed in both skills
- Tradeoffs explicitly acknowledged and deferred (transitions, rename, helpers,
  audit trail, migration)
- `list-tickets` correctly scopes itself read-only with no sub-agents
- Cycle detection specified for hierarchy rendering
- Modelled on `review-adr` — good pattern fidelity
- Plugin registration requires no changes

**Findings**:

- 🟡 major/medium: Implicit contract between template trailing-comment format and
  runtime hint parser
- 🟡 major/high: Frontmatter parsing logic duplicated across bash scripts and two
  LLM prompts
- 🔵 minor/high: Body status-line sync borrows closed-vocabulary pattern into
  open-vocabulary context
- 🔵 minor/medium: `ticket_id` edits without rename create persistent
  filename/frontmatter divergence
- 🔵 minor/medium: Legacy-ticket filter semantics silently exclude rather than
  surface the asymmetry
- 🔵 minor/high: Integration verification exercises structural form but not
  cross-skill data-flow invariants
- 🔵 suggestion/medium: Tag array round-tripping specified but not owned by a
  single component

### Code Quality

**Summary**: The plan is well-structured with clear TDD ordering, strong
location pointers to exemplar skills, and explicit 'not doing' boundaries that
keep scope tight. However, several specification gaps create risk of
inconsistent behaviour across the two SKILL.md prompts — notably the filter
parsing contract for list-tickets, the 'hint extraction' mechanism for template
defaults, and the edit-application step for update-ticket. Several smaller
contradictions across sibling specs will produce minor maintenance pain.

**Strengths**:

- TDD order explicit and repeated in three places
- `What We're NOT Doing` enumerates six concrete exclusions
- Pattern sources cited by file AND line range
- Quality guidelines collected into explicit sections on both skills
- Scenario numbering with descriptive titles acts as a readable test index
- Legacy ticket compatibility called out as first-class

**Findings**:

- 🟡 major/high: Filter-expression grammar under-specified
- 🟡 major/high: Template hint extraction mechanism unspecified — both skills
  share fragile contract
- 🟡 major/medium: Edit-application step vague about frontmatter block rewrite
- 🟡 major/medium: Inconsistent error-handling contract for malformed frontmatter
  across sibling skills
- 🔵 minor/high: Cycle-detection requirement has no eval
- 🔵 minor/high: Inconsistent quoting convention for parent field value
- 🔵 minor/medium: Body status-sync regex narrow; title-casing ambiguous
- 🔵 minor/medium: Hierarchy mode behaviour on legacy tickets missing parent
  unstated
- 🔵 minor/medium: `grep -iE "transition..."` automated check is brittle
- 🔵 minor/high: Mixed interpretation-echoing contract across scenarios
- 🔵 minor/medium: "em-dash or blank" for missing fields under-specified

### Test Coverage

**Summary**: The 14+22 approach evals cover the principal happy paths, key error
modes, and the legacy-ticket coexistence requirement — a solid TDD skeleton.
However, several high-risk behaviours called out in the Quality Guidelines (
parent cycle detection in list-tickets, multi-op combined edits in
update-ticket, YAML tag-array re-serialisation edge cases, body-status sync with
case-insensitive matching) lack corresponding eval scenarios. The Phase 1
regression suite is a strong guard for the three bash scripts but provides zero
coverage for the SKILL.md prompts.

**Strengths**:

- Explicit TDD framing
- Legacy-schema coexistence tested directly (3.1 Scenario 10, 3.2 Scenario 19)
- Error paths have real scenarios (malformed, missing, ambiguous, decline)
- No-op detection provides mutation-testing pressure
- Phase 1 regression suite has strong coverage of its own surface

**Findings**:

- 🟡 major/high: Parent-cycle detection specified but has no eval
- 🟡 major/high: Multi-op combined edits have no eval scenario
- 🟡 major/medium: Body-status sync scenario under-specifies matching rules
- 🟡 major/medium: Tag add/remove evals miss key edge cases
- 🟡 major/medium: Tag-array parsing only tested on a single happy shape
- 🟡 major/medium: Read-only-field warning path lacks decline scenario
- 🟡 major/medium: Parent-value canonicalisation across padded/unpadded forms
  untested
- 🔵 minor/medium: NL filter parser has no negative-case eval
- 🔵 minor/medium: Confirmation-token acceptance under-asserted
- 🔵 minor/high: Integration verification's only real end-to-end assertion is a
  single manual happy-path
- 🔵 minor/high: The 'no transition enforcement' grep check is theatre
- 🔵 minor/medium: No eval combines hierarchy rendering with legacy tickets
  missing parent

### Correctness

**Summary**: The plan lays out scenario evals with reasonable coverage of happy
paths and several edge cases, but multiple correctness gaps remain: the listing
schema assumes a `title` frontmatter field that does not exist in
`templates/ticket.md`; parent-matching and number-padding semantics are
under-specified; array re-serialisation for tag add/remove does not account for
quoted elements or block-style YAML arrays; and body-sync is defined only for
`**Status**:` despite the template also carrying `**Type**:`, `**Priority**:`,
`**Author**:`.

**Strengths**:

- Malformed frontmatter handled in two distinct modes aligned with script
  behaviour
- Legacy ticket coexistence considered explicitly
- No-op detection and ambiguous-number handling well specified
- Read-only field warnings avoid silent data corruption without blocking
- Confirmation preview + decline path makes write flow auditable and reversible
- Cycle detection called out in guidelines

**Findings**:

- 🟡 major/high: Title column source unresolved — new-schema tickets have no
  `title:` frontmatter
- 🟡 major/high: Body sync covers only `**Status**:` but template carries
  `**Type**:`/`**Priority**:`/`**Author**:`
- 🟡 major/high: Tag re-serialisation does not account for quoted elements,
  spaces, or block-style YAML arrays
- 🟡 major/medium: Parent-field format and matching semantics under-specified
- 🟡 major/medium: Multi-op argument grammar ambiguous for mixed structured and
  tag ops
- 🔵 minor/high: Title-casing of hyphenated status values undefined
- 🔵 minor/high: Parent cycle detection required by guidelines but no eval
- 🔵 minor/medium: Template-comment parsing for status hints brittle and
  unspecified
- 🔵 minor/medium: Path-like detection heuristic may misclassify bare filenames
- 🔵 minor/medium: Filter-term disambiguation rules informal; collide with
  legitimate value strings
- 🔵 minor/medium: Re-entry flow after 'ask what to change' prompt unspecified

### Standards

**Summary**: The plan's frontmatter spec (fields, order,
disable-model-invocation, inline allowed-tools, accelerator: agent prefixes) and
overall structural ordering faithfully mirror the Phase 2 exemplars and carry
forward every standards fix from the pass-1 Phase 2 review. However, the plan
omits several conventions visible in create-ticket/extract-tickets SKILL.md —
notably the H1 skill heading, the canonical `## Ticket Template` heading with
framing paragraph, the canonical agent-fallback phrasing, and the plain
multi-line description scalar convention.

**Strengths**:

- Frontmatter fields in same order as both Phase 2 exemplars
- `allowed-tools` inline (not block scalar)
- Agent fallback list carries accelerator: prefix
- Configuration preamble ordered correctly
- Path injection uses bold-label convention
- Instructions injection at end of file
- Template section placed in body between path injection and skill prose
- argument-hint values are quoted strings
- Integration verification includes agent prefix grep

**Findings**:

- 🔵 minor/high: Template section spec omits `## Ticket Template` heading and
  framing paragraph
- 🔵 minor/high: Agent fallback spec does not require canonical phrasing
- 🔵 minor/medium: No H1 skill heading specified
- 🔵 minor/medium: list-tickets allowed-tools includes `tickets/scripts/*` but
  skill invokes no Phase 1 script
- 🔵 minor/medium: Plan does not explicitly require plain multi-line description
  scalars with 2-space indent
- 🔵 minor/high: `grep -r` count assertion in Subphase 3.3 is fragile

### Usability

**Summary**: The plan defines two skills with a strong commitment to feedback
affordances — echo-back, diff preview, explicit confirmation, no-op detection,
template-derived hints. However, natural-language parsing rules in both skills
concentrate significant ambiguity into free-text arguments with no
disambiguation protocol, argument-hints are under-specified relative to the
breadth of supported forms, and several error/abort paths drop the user into a
dead end without actionable next steps.

**Strengths**:

- Explicit echo-back of interpreted filter above the table
- Unified diff + explicit y/n confirmation before any write
- No-op detection and read-only-field warnings
- NL ops echoed as `Interpreted as: <field> → <value>`
- Bare `/list-tickets` produces a useful full table
- Template-derived hints adapt to user-overridden templates
- Template defaults treated as hints, not a closed set

**Findings**:

- 🟡 major/high: NL filter parsing has ambiguous cases producing wrong filters
  silently
- 🟡 major/high: Bare `/update-ticket` does not teach the user what ops are valid
- 🟡 major/high: Field-name-only op collides with natural-language arguments
- 🔵 minor/high: Confirmation prompt response tolerance unspecified
- 🔵 minor/high: Hierarchy vs `under 0042` distinction subtle; composition
  undocumented
- 🔵 minor/high: Tag ops without `add`/`remove` keyword are ambiguous
- 🔵 minor/medium: Malformed-frontmatter abort says 'add a `---`' but does not
  show where
- 🔵 minor/medium: Argument hints terse; do not signal natural-language breadth
- 🔵 minor/medium: Legacy tickets render as wall of em-dashes
- 🔵 minor/medium: Ambiguous number match aborts without offering numeric
  selection
- 🔵 minor/medium: Fallback behaviour when template has no comment annotations
  unspecified

### Compatibility

**Summary**: The plan is largely compatibility-conscious: it explicitly calls
out the legacy 29 `adr-creation-task` tickets, commits to non-enforcement of
status transitions, avoids hardcoding type/status/priority values, and forbids
modification of Phase 1 scripts and Phase 2 skills. However, several
contract-level compatibility gaps remain: Scenario 14's legacy title-fallback
claim is inconsistent with actual legacy schema; NL filter shorthands silently
exclude legacy tickets; the dependency on `ticket-read-field.sh`'s
raw-YAML-array return value is not formalised; and `update-ticket`'s body sync
under-matches on legacy tickets and title-cases arbitrary custom values.

**Strengths**:

- Explicit non-enforcement of status transitions
- Legacy ticket inclusion is first-class
- Template-derived hints non-authoritative
- Filename NNNN prefix declared authoritative
- `What We're NOT Doing` forbids changes to Phase 1 scripts, template, Phase 2
  skills
- Both skills consume `ticket-read-field.sh` read-only and follow Phase 2
  conventions

**Findings**:

- 🟡 major/high: Legacy title-fallback claim contradicts actual legacy ticket
  schema
- 🟡 major/high: Natural-language filter shorthands silently exclude entire
  legacy corpus
- 🟡 major/medium: Body Status-line sync title-cases arbitrary values; misses
  legacy body conventions
- 🔵 minor/high: Array re-serialisation contract with `ticket-read-field.sh` not
  formalised
- 🔵 minor/high: Conflation of "empty directory" and "directory does not exist"
- 🔵 minor/medium: Plugin scanning of `skills/tickets/` includes non-skill
  `scripts/` subdirectory
- 🔵 minor/medium: Template-comment parser coupled to shipping template's exact
  formatting
- 🔵 suggestion/medium: Read-only warning for `date` field mentions compatibility
  concerns that don't apply

### Safety

**Summary**: The plan demonstrates solid safety fundamentals: the write path is
gated by a diff preview and explicit y/n confirmation, no-op detection
short-circuits spurious writes, malformed frontmatter aborts cleanly, ambiguous
globs abort safely, and git/jj is a reasonable audit trail. However, several
write-path specifications are under-tightened in ways that could silently
corrupt ticket content: the body `**Status**:` regex is not disambiguated
against multiple matches, the tags re-serialisation spec does not pin format
fidelity, and the read-only field warning for `ticket_id` understates downstream
breakage.

**Strengths**:

- Always-on diff preview + explicit y confirmation; byte-for-byte verifiable
- No-op detection avoids unnecessary writes
- Ambiguous number aborts cleanly
- Malformed frontmatter aborts; refuses auto-repair
- `list-tickets` read-only by construction
- Legacy ticket compatibility: no silent migration or coercion
- `What We're NOT Doing` establishes firm safety boundaries
- Git/jj appropriate as audit trail
- NL interpretations echoed for user verification

**Findings**:

- 🟡 major/high: Body `**Status**:` sync regex ambiguous; can silently corrupt
  body content
- 🟡 major/high: Tag array parse/mutate/re-serialise loop has no format-fidelity
  guarantee
- 🟡 major/medium: `ticket_id` warning understates cascading breakage
- 🔵 minor/medium: No atomicity guarantee for multi-line edits
- 🔵 minor/medium: Affirmative-response matching under-specified
- 🔵 minor/medium: Combined multi-op diff has no per-op opt-out
- 🔵 minor/low: Cycle-detection guarantee for hierarchy rendering asserted but
  not evaluated
