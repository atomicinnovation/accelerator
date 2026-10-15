---
type: "work-item-review"
id: "0332-push-pr-descriptions-to-github-unwrapped-review-1"
title: "Work Item Review: Push PR Descriptions to GitHub Unwrapped"
date: "2026-10-13T17:27:43+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0332"
work_item_id: "0332"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-10-13T18:42:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Push PR Descriptions to GitHub Unwrapped

**Verdict:** COMMENT

The work item is a single, well-sized story. Its Requirements define the
replaced byte span precisely, and most of its Acceptance Criteria use
literal input and output strings. It can go ahead as it stands, but it
could be improved: the one major gap is that no fixture covers nested
or lazy container prefixes, which are the cases the Technical Notes name
as undocumented parser behaviour. The other findings are bookkeeping
(`relates_to`, Dependencies entries) and wording.

### Cross-Cutting Themes

- **`relates_to` is out of step with References** (flagged by:
  completeness, dependency) — the body names 0071 (the contract this
  story replaces) and 0152 (future callers), but the frontmatter lists
  neither.
- **Coordination with 0320 is implicit** (flagged by: scope,
  dependency) — this story brings in the first Markdown crate. Nothing
  says that 0320 should reuse that crate rather than add a second
  parser.
- **The event-stream oracle uses the implementation's own parser**
  (flagged by: clarity, testability) — the criterion does not name the
  parser or its options. Because the same parser sits on both sides of
  the comparison, it cannot detect divergence from GitHub's
  `cmark-gfm`, which is the main residual risk.
- **The `meta/prs/` verification is loosely bounded** (flagged by:
  scope, testability) — the corpus can grow, the handling of a failure
  that cannot be fixed is unspecified, and the outcome of the run is not
  recorded anywhere.

### Findings

#### Major

- 🟡 **Testability**: Nested and lazy container prefixes have no concrete fixture
  **Location**: Acceptance Criteria
  The span definition covers container prefixes and lazy continuation,
  and the byte ranges of SoftBreak inside containers are undocumented.
  Despite that, the only container fixtures are a single-level
  blockquote and a list criterion that gives no literal strings.

#### Minor

- 🔵 **Completeness / Dependency**: `relates_to` omits 0071 and 0152
  **Location**: Frontmatter: relates_to
  References names both items as related, but the typed links leave them
  out.
- 🔵 **Dependency**: The new Markdown crate and its cargo-deny gate are not listed as a dependency
  **Location**: Dependencies
  `pulldown-cmark` is not yet in the workspace, and if cargo-deny
  rejects it, the parse-based approach is blocked.
- 🔵 **Dependency**: Reliance on GitHub's `cmark-gfm` rendering is not named as an external coupling
  **Location**: Dependencies
  The premise of the story rests on GitHub-side behaviour, which is
  recorded only in Context and Technical Notes.
- 🔵 **Dependency**: Ordering and choice of crate with 0320 are left implicit
  **Location**: Drafting Notes
  The two stories could each add a different Markdown crate.
- 🔵 **Scope**: The corpus-verification criterion leaves the scope open-ended
  **Location**: Acceptance Criteria
  The corpus can grow, and parser divergences found in it could pull an
  unknown amount of extra work into the story.
- 🔵 **Testability**: The event-stream oracle cannot detect divergence from GitHub
  **Location**: Acceptance Criteria
  The same parser and options are applied to both sides, so a wrong
  option set gives the same result on both and the check still passes.
- 🔵 **Testability**: The unconditional, no-configuration behaviour has no criterion
  **Location**: Requirements
  A config toggle added later would pass every criterion that is listed.
- 🔵 **Testability**: The byte-identical construct check has no defined comparison procedure
  **Location**: Acceptance Criteria
  Once soft breaks are removed, the constructs sit at different offsets
  in the output, and the criterion does not say how to match them up.
- 🔵 **Testability**: The one-off `meta/prs/` verification leaves no checkable evidence
  **Location**: Acceptance Criteria
  Nothing says where the result is recorded.
- 🔵 **Clarity**: The "no newline" criterion appears to conflict with preserving hard breaks and alert markers
  **Location**: Acceptance Criteria
  Read literally, AC1 fails on any fixture that contains a hard break or
  an alert.
- 🔵 **Clarity**: The quoted phrases in Drafting Notes have no stated source
  **Location**: Drafting Notes
  The phrases "Never wrap", "Regardless of configuration" and
  "Consistent across built-in templates" appear nowhere else in the
  work item.
- 🔵 **Clarity**: "GFM event stream" and "`Text` events" are used without naming the parser
  **Location**: Acceptance Criteria
  The option set decides whether alerts and footnotes appear in the
  stream at all.

#### Suggestions

- 🔵 **Completeness**: Open Questions is empty and does not say that none remain
  **Location**: Open Questions
- 🔵 **Scope**: Two stories solve soft-break unwrapping independently
  **Location**: Drafting Notes
- 🔵 **Testability**: The table-interrupts-paragraph divergence is named as a risk but has no fixture
  **Location**: Acceptance Criteria
- 🔵 **Clarity**: The subject of "when it is unwrapped" is not named
  **Location**: Acceptance Criteria
- 🔵 **Clarity**: "In practice" leaves it unclear whether the preserved-constructs list is binding
  **Location**: Requirements
- 🔵 **Clarity**: "Configuration" and "conventions" are used for the same thing
  **Location**: Summary

### Strengths

- ✅ The replaced span is defined at byte level, and the definition
  includes the lazy-continuation exception.
- ✅ Most criteria give literal inputs and expected outputs. The
  event-stream and idempotence checks add a property that covers every
  fixture.
- ✅ The replacement of 0071's byte-for-byte contract is named
  outright and limited to "differs at soft breaks only".
- ✅ Out-of-scope items are stated with reasons: `review-pr`,
  `respond-to-pr`, frontmatter, templates, the Linear path and
  opt-outs.
- ✅ The GFM edge cases belong in the story: an unwrap without them
  would break alerts and footnotes on GitHub.
- ✅ Assumptions record what happens if they turn out wrong, and the
  evidence behind them (a scan of 115 descriptions).
- ✅ The upstream prerequisites, 0197 and 0071, are both done, so
  "Blocked by: none" is accurate.

### Recommended Changes

1. **Add literal fixtures for container prefixes** (addresses: Nested and
   lazy container prefixes have no concrete fixture)
   Rewrite the list criterion with literal strings, and add these
   fixtures: `> - item\n>   continued`, `- a\n  - nested\n    more`,
   `> first\nsecond` and `- [ ] task\n  wrapped`.
2. **Make AC1 refer to soft breaks** (addresses: The "no newline"
   criterion appears to conflict)
   Say "contains no soft line break", or exclude hard breaks and the
   alert marker line explicitly.
3. **Name the parser and options in the event-stream criterion**
   (addresses: Parser not named; Oracle cannot detect divergence)
   State that the stream comes from the unwrap's parser and GFM options.
   Optionally, have the one-off verification also compare the HTML that
   `POST /markdown` returns for the input and the output.
4. **Bound and record the `meta/prs/` verification** (addresses:
   Corpus criterion is open-ended; leaves no checkable evidence)
   Pin the corpus to a count or a commit, and send any failure that
   cannot be fixed to a follow-up work item. Record the outcome in the
   PR description.
5. **Add a criterion for the no-configuration requirement** (addresses:
   No criterion for unconditional behaviour)
   For example: the PATCHed body is the same with and without
   `.editorconfig` and `.accelerator/` config, and `update-body`
   exposes no unwrap option.
6. **Fill in Dependencies and `relates_to`** (addresses: `relates_to`
   omissions; new crate; GitHub coupling; 0320 coordination)
   Add `work-item:0071` and `work-item:0152` to `relates_to`. Add these
   entries to Dependencies: External: `pulldown-cmark` (cargo-deny);
   External: GitHub PR-body rendering; and a note that 0320 reuses this
   story's crate.
7. **Tighten the wording** (addresses: quoted phrases without a source;
   whether the preserved list is binding; configuration vs conventions;
   empty Open Questions; the subject of "unwrapped")
   Name PP-885 as the source of the quoted phrases. Say whether the
   preserved-constructs list is a consequence of the span rule or a
   guarantee in its own right. Use "conventions" consistently. Write
   "None" under Open Questions. Say that the fixture criteria test the
   unwrap operation directly.
8. **Optionally add a fixture for a table that interrupts a paragraph**
   (addresses: Table-interrupts-paragraph divergence not fixtured)
   Check the expected output once, by hand, against GitHub.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: This is a clear, precise work item. It names the actor,
defines the span at byte level, and keeps one consistent intent
throughout. The remaining issues are small: the wording of AC1
alongside the rules for preserving hard breaks and alert markers,
quoted phrases in Drafting Notes with no stated source, and terms tied
to a parser or entry point that is never named.

**Strengths**:
- The replaced span is defined exactly, including the
  lazy-continuation exception.
- The replacement of the 0071 contract is named outright and limited
  in scope.
- Scope boundaries come with reasons.
- Concrete fixtures pin down behaviour that prose would leave open.
- Drafting Notes explain the choices that are not obvious.

**Findings**:
- 🔵 minor (medium) — Acceptance Criteria — The "no newline" criterion
  appears to conflict with preserving hard breaks and alert markers.
  Read literally, a wrapped fixture that contains a hard break or an
  alert fails AC1 but passes the later criteria. Suggestion: limit AC1
  to soft breaks.
- 🔵 minor (medium) — Drafting Notes — The quoted phrases have no
  stated source. "Never wrap", "Regardless of configuration" and
  "Consistent across built-in templates" appear nowhere else.
  Suggestion: name PP-885 or the original request, or rephrase them as
  decisions.
- 🔵 minor (medium) — Acceptance Criteria — "GFM event stream" and
  "`Text` events" are pulldown-cmark concepts used without naming the
  parser or its options. Suggestion: state in the criterion which
  parser and options produce the stream.
- 🔵 suggestion (medium) — Acceptance Criteria — The subject of "when
  it is unwrapped" is not named. It is unclear whether the fixtures are
  verified through the CLI or against the unwrap directly. Suggestion:
  name the unwrap as an operation in its own right, or say that the
  fixtures are observed through the PATCHed body.
- 🔵 suggestion (low) — Requirements — "In practice" leaves it unclear
  whether the preserved-constructs list is binding or only gives
  examples. Suggestion: say which.
- 🔵 suggestion (low) — Summary — "Configuration" and "conventions"
  are both used for what governs the artefact's wrapping, while Context
  names other causes too. Suggestion: use one term.

### Completeness

**Summary**: This is an unusually complete story. Every section is
present and well filled in, and the frontmatter is valid. The only gaps
are bookkeeping: `relates_to` leaves out 0071 and 0152, and the empty
Open Questions section does not say whether it is empty on purpose.

**Strengths**:
- The Summary is a proper user story, followed by the mechanism and
  what stays unchanged.
- Context traces the whole chain from cause to effect, including the
  callers that inherit the change and the contract it replaces.
- The Requirements are detailed enough to start implementing without
  follow-up questions.
- 13 Given/When/Then criteria cover every construct in the
  Requirements, plus checks that apply to the whole output.
- Assumptions and Drafting Notes record the evidence and reasoning
  behind each scoping choice.

**Findings**:
- 🔵 minor (high) — Frontmatter: relates_to — `relates_to` omits 0071
  and 0152. Suggestion: add both, and consider marking the link to 0071
  as a supersession.
- 🔵 suggestion (medium) — Open Questions — The section is empty and
  does not say that none remain. Suggestion: write "None".

### Dependency

**Summary**: The work item makes its couplings visible: callers,
future inheritors, the 0071 contract and its sibling 0320. "Blocked by:
none" is accurate. The gaps all concern couplings that are recorded
only in Technical Notes or Drafting Notes and not in Dependencies.

**Strengths**:
- The downstream inheritors are named: `raise-pr` and 0152.
- The replacement of the 0071 contract is explicit.
- The relationship with 0320 is stated.
- The consumers left out of scope are named, with reasons.
- The upstream prerequisites, 0197 and 0071, are done.

**Findings**:
- 🔵 minor (high) — Dependencies — The new Markdown crate and its
  cargo-deny gate are not listed. Suggestion: add "External:
  pulldown-cmark (new workspace dependency; must pass cargo-deny)".
- 🔵 minor (medium) — Dependencies — Reliance on GitHub's `cmark-gfm`
  rendering is not named as an external coupling. Suggestion: record
  it, with a reference to the GitHub Docs link.
- 🔵 minor (medium) — Drafting Notes — Ordering and choice of crate
  with 0320 are left implicit. Suggestion: say that 0320 reuses this
  story's crate, or add "Informs: 0320".
- 🔵 suggestion (medium) — Frontmatter: relates_to — 0071 and 0152 are
  missing. Suggestion: add both.

### Scope

**Summary**: This is one coherent story of the right size, with
explicit boundaries. The edge cases cannot be split off, because an
unwrap without them would break rendering. The concerns are minor: the
corpus criterion is open-ended, and the story deliberately solves the
same problem a second time alongside 0320.

**Strengths**:
- Every requirement serves one purpose.
- Out-of-scope items are named, with reasons.
- The work sits in a single component.
- The GFM edge cases belong in this story, not in follow-ups.
- The replacement of the 0071 contract is acknowledged.
- The kind is right: more than a chore and less than an epic.

**Findings**:
- 🔵 minor (medium) — Acceptance Criteria — The corpus-verification
  criterion leaves the scope open-ended. Suggestion: pin the corpus to
  a count or a commit, and send failures that cannot be fixed to a
  follow-up.
- 🔵 suggestion (low) — Drafting Notes — Two stories solve soft-break
  unwrapping independently. Suggestion: add a note on sequencing so
  that whichever lands second reuses or extracts the first one's
  unwrap.

### Testability

**Summary**: This work item is unusually testable. Its criteria use
literal inputs and outputs, and the event-stream and idempotence checks
cover every fixture. The gaps are: no fixture for nested or lazy
container prefixes, no criterion for the no-configuration requirement,
and an event-stream oracle that cannot detect divergence from GitHub.

**Strengths**:
- Literal inputs and exact expected outputs give each test a definite
  pass or fail.
- The event-stream and idempotence checks apply to every fixture.
- The CRLF criterion is checked by comparison with the `\n` case.
- Both observable effects are covered: the PATCHed body and the
  unchanged file.
- The corpus verification stays out of the suite, which keeps the
  suite hermetic.

**Findings**:
- 🟡 major (medium) — Acceptance Criteria — Nested and lazy container
  prefixes have no concrete fixture. Suggestion: add
  `> - item\n>   continued`, `- a\n  - nested\n    more`,
  `> first\nsecond` and `- [ ] task\n  wrapped`, and rewrite the list
  criterion with literal strings.
- 🔵 minor (medium) — Acceptance Criteria — The event-stream oracle
  cannot detect divergence from GitHub. Suggestion: compare the HTML
  from `POST /markdown` in the one-off verification, or pin the test
  parser's options independently of the production constant.
- 🔵 minor (high) — Requirements — The unconditional, no-configuration
  behaviour has no criterion. Suggestion: require the same output with
  and without config, and no unwrap option in `--help`.
- 🔵 minor (medium) — Acceptance Criteria — The byte-identical
  construct check has no defined comparison procedure. Suggestion:
  compare whole documents on a fixture with no soft breaks outside the
  constructs, and add a second fixture with wrapped neighbours and a
  literal expected output.
- 🔵 minor (medium) — Acceptance Criteria — The one-off `meta/prs/`
  verification leaves no checkable evidence. Suggestion: record the
  counts and the fixtures it produced in the PR description, and name
  the command used.
- 🔵 suggestion (low) — Acceptance Criteria — The
  table-interrupts-paragraph divergence is named as a risk but has no
  fixture. Suggestion: add a fixture whose expected output is checked
  once against GitHub.

## Re-Review (Pass 2) — 2026-10-13T17:41:01+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: Nested and lazy container prefixes have no concrete fixture — Resolved
- 🔵 **Completeness / Dependency**: `relates_to` omits 0071 and 0152 — Resolved
- 🔵 **Dependency**: New Markdown crate and cargo-deny gate not listed — Resolved
- 🔵 **Dependency**: GitHub rendering not named as an external coupling — Resolved
- 🔵 **Dependency**: Ordering and crate choice with 0320 left implicit — Partially resolved (the Informs line is narrower than the Drafting Notes)
- 🔵 **Scope**: Corpus-verification criterion open-ended — Partially resolved (fixable failures still have no size limit)
- 🔵 **Testability**: Event-stream oracle cannot detect divergence from GitHub — Resolved (by `POST /markdown`, which introduces a new issue below)
- 🔵 **Testability**: No criterion for the no-configuration requirement — Partially resolved (2 of 4 sources covered)
- 🔵 **Testability**: Byte-identical check lacks a procedure — Resolved
- 🔵 **Testability**: One-off verification leaves no evidence — Resolved
- 🔵 **Clarity**: "No newline" in AC1 conflicts with hard breaks — Resolved
- 🔵 **Clarity**: Quoted phrases have no source — Resolved
- 🔵 **Clarity**: Event stream used without naming the parser — Resolved
- 🔵 **Clarity**: Subject of "when it is unwrapped" not named — Partially resolved ("directly" names no entry point)
- 🔵 **Clarity**: Status of the "In practice" list unclear — Resolved
- 🔵 **Clarity**: "Configuration" and "conventions" used for the same thing — Resolved
- 🔵 **Completeness**: Open Questions empty — Resolved
- 🔵 **Scope**: Two stories solve soft-break unwrapping independently — Resolved
- 🔵 **Testability**: Table interrupting a paragraph has no fixture — Resolved

### New Issues Introduced

- 🟡 **Testability**: The continuation-line criterion contradicts CommonMark. `- x`, `1. x`, `# x` and `> x` interrupt a paragraph, so they are not continuation lines; only lines like `2. x`, `-x`, `#5` and `    > x` are. The criterion needs literal fixtures, plus the reverse case `foo\n- bar`, which must stay unchanged.
- 🟡 **Testability**: The corpus HTML comparison names no `POST /markdown` mode and no normalisation. `gfm` mode renders newlines as `<br>`, so wrapped input can never match the unwrapped output. The comparison needs `markdown` (document) mode, with whitespace normalised outside `<pre>`.
- 🔵 **Testability**: Unclear whether the corpus criterion is a gate or a report.
- 🔵 **Testability**: Interleaved and 80-column fixtures have no literal expected output, and no fixture mixes hard and soft breaks in one paragraph.
- 🔵 **Testability**: Event-stream normalisation may not absorb the whitespace trimmed around a span.
- 🔵 **Testability**: The no-configuration criterion omits CLAUDE.md and the custom template, and the `--help` check and the manual table check are open-ended.
- 🔵 **Dependency**: `POST /markdown` (an authenticated token and about 230 calls) is not listed.
- 🔵 **Dependency**: No minimum `pulldown-cmark` version for `ENABLE_GFM` alerts.
- 🔵 **Dependency**: Consumers of the 0071 contract are not listed (`describe-pr` SKILL.md says the description shown is "what gets posted").
- 🔵 **Dependency**: The 0320 Informs line says "crate" while Drafting Notes say "unwrap", and 0320 has no reciprocal link.
- 🔵 **Scope**: Fixable corpus failures have no size limit.
- 🔵 **Clarity**: "The PR description" is ambiguous, and so is "the Markdown crate this story adds".
- 🔵 **Clarity**: "Can fix" versus "cannot fix" has no stated rule.
- 🔵 **Clarity**: Neither "directly" nor "each fixture above" says what it refers to.
- 🔵 **Clarity**: "Push", "post" and "PATCH" all name one step, and the manual table check has no stated owner.

### Assessment

The first pass's major finding and 14 of its other 18 findings are
resolved, and completeness is now clean. The edits introduced two new
major findings, both in criteria added this pass: the continuation-line
fixtures, and the mode for the corpus HTML comparison. Two majors meet
the REVISE threshold. Both have small, concrete fixes, and the minor
findings are mostly about wording.

## Re-Review (Pass 3) — 2026-10-13T18:36:15+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: Continuation-line criterion contradicts CommonMark — Resolved
- 🟡 **Testability**: Corpus HTML comparison names no mode or normalisation — Resolved
- 🔵 **Testability**: Corpus criterion is a gate or a report — Partially resolved (the pass condition now contradicts itself; see below)
- 🔵 **Testability**: No-configuration criterion omits CLAUDE.md and the custom template — Resolved
- 🔵 **Dependency**: `POST /markdown` is not listed — Resolved
- 🔵 **Dependency**: No minimum `pulldown-cmark` version — Resolved
- 🔵 **Dependency**: Consumers of the 0071 contract are not listed — Partially resolved (the `describe-pr` wording is now in scope, but 0071 is still absent from Dependencies)
- 🔵 **Dependency**: The 0320 Informs line is narrower than the Drafting Notes — Still present
- 🔵 **Scope**: Fixable corpus failures have no size limit — Still present
- 🔵 **Clarity**: "The PR description" is ambiguous — Resolved
- 🔵 **Clarity**: "Can fix" versus "cannot fix" has no stated rule — Partially resolved (the classes are named, but there is no way to assign a failure to one)
- 🔵 **Clarity**: Neither "directly" nor "each fixture above" says what it refers to — Still present
- 🔵 **Testability**: Interleaved and 80-column fixtures have no literal expected output — Still present
- 🔵 **Clarity**: "Push", "post" and "PATCH" all name one step — Not re-flagged

### New Issues Introduced

- 🟡 **Clarity**: The phrase "an indented `> x`" sets no indentation threshold. A `>` indented by one to three spaces still interrupts a paragraph; only four or more spaces make the line continuation text.
- 🟡 **Clarity**: The corpus pass condition reads "passes or has a linked follow-up", which lets span-logic failures be deferred even though an earlier sentence says they are fixed in this story. It also gives no way to attribute a failure to one class or the other.
- 🔵 **Testability**: The new `describe-pr` wording requirement has no criterion.
- 🔵 **Testability**: Inline HTML is guaranteed preserved, but no fixture covers inline HTML that spans a line ending.
- 🔵 **Testability**: The CRLF criterion names no fixture; it could instead be a property over every fixture.
- 🔵 **Clarity**: The "one physical line per paragraph" target in Drafting Notes conflicts with preserved hard breaks.
- 🔵 **Clarity**: The criterion does not say what happens if the by-hand table check contradicts its fixture.
- 🔵 **Clarity**: The Assumptions say "opt-in" where the Requirements say "opt-out", and "GFM" is never expanded.
- 🔵 **Dependency**: Neither 0071 nor 0152 appears in Dependencies.

### Assessment

Both pass-2 majors are resolved, and dependency, scope and testability
now report minor findings only. Clarity raised 2 new majors, which
again meets the REVISE threshold; both come from wording added in pass
2. Each needs a fix of about one sentence: give the four-space
threshold, and restrict follow-ups to parser divergence with a
mechanical attribution rule. A failure counts as parser divergence only
if `pulldown-cmark`'s HTML for the input already differs from GitHub's.

## Re-Review (Pass 4) — 2026-10-13T18:42:18+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: "An indented `> x`" threshold — Resolved
- 🟡 **Clarity**: Corpus pass condition contradicts itself — Partially resolved (attribution rule added, but it misclassifies; see below)
- 🔵 **Testability**: `describe-pr` wording has no criterion — Resolved (pass condition loose; see below)
- 🔵 **Testability**: Inline HTML has no fixture — Resolved
- 🔵 **Testability**: CRLF criterion has no fixture — Resolved
- 🔵 **Clarity**: "One physical line" target conflicts with hard breaks — Resolved
- 🔵 **Dependency**: 0320 coupling is one-way in Dependencies — Still present
- 🔵 **Scope**: The corpus fix loop has no size limit — Still present
- 🔵 **Clarity**: "In its own right" and "GFM" are unexplained — Still present

### New Issues Introduced

- 🟡 **Clarity**: The preamble misorders the criteria: the `describe-pr` criterion is third, not fourth.
- 🟡 **Testability**: The attribution rule compares `pulldown-cmark` HTML with GitHub HTML, which always differ (anchors, `user-content-` ids, sanitisation). Almost every failure would therefore be classed as parser divergence.
- 🟡 **Testability**: The end-to-end criterion ("no soft line break") would pass even with a naive or wrong unwrap. It needs an exact expected `body`.
- 🔵 **Testability**: Joined setext headings change GitHub's anchor ids, so the HTML comparison would fail on an expected difference.
- 🔵 **Testability**: The footnote and interleaved fixtures have no literal input or output.
- 🔵 **Testability**: There are no fixtures for `foo\n   bar`, `[a\nb](u)`, `~~a\nb~~` or `> a\n  b`.
- 🔵 **Testability**: Byte preservation is not checked across the corpus, and no criterion compares `<br>` counts in `gfm` mode.
- 🔵 **Clarity**: "Body file" and "artefact" are conflated, and the criterion does not say whether frontmatter is stripped before the corpus check.
- 🔵 **Clarity**: The table criterion never states the expected rendering.
- 🔵 **Dependency**: The criterion does not name the crate that will host the unwrap or the checks beyond `cargo-deny`.

### Assessment

Each pass resolves its majors, and new majors keep appearing in
wording that was added in the previous pass. Most of the churn traces
to the `meta/prs/` corpus criterion, which has grown into a small
verification protocol. The three new majors have mechanical fixes:

- reorder the criteria;
- attribute a failure to span logic when the unwrap's own event-stream
  or idempotence check fails;
- give the end-to-end criterion an exact expected `body`.

The minor findings now concern how deep the fixtures go, not whether
the story is ready.
