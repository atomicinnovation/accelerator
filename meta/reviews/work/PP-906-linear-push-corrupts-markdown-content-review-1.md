---
type: "work-item-review"
id: "PP-906-linear-push-corrupts-markdown-content-review-1"
title: "Work Item Review: Linear Push Corrupts Markdown Content"
date: "2026-11-23T15:34:30+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:PP-906"
work_item_id: "PP-906"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-11-23T16:43:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Linear Push Corrupts Markdown Content

**Verdict:** REVISE

PP-906 is a well-built bug: every section is filled in, both defects carry
exact reproductions tied to capture fragments, and the acceptance criteria are
split into CI, pinned-capture and manual lanes, with PP-880's equivalence as
the test oracle. The problems sit in Required behaviour. The digit-escape
trigger and the dotted-token matching rule can each be read two ways, and the
acceptance criteria pin only the narrow, captured cases, so an over- or
under-escaping implementation would still pass. Dependencies also leaves out
the external couplings the fix depends on: Linear's parser and linkifier, and
credentialed workspace access for the live captures.

### Cross-Cutting Themes

- **Digit-escape trigger is wider than the defect** (flagged by: clarity,
  scope, testability) — Required behaviour escapes `<digits>.` "at the start
  of a continuation line", but Defect 1, `X09` and the acceptance criterion
  cover only a lone number inside a list item. Nothing says what happens to
  top-level lines, lines with trailing text, or genuine ordered lists.
- **Dotted-token rule is undefined beyond the capture** (flagged by: clarity,
  testability) — "each bare token that Linear links, as the capture records,
  and in no other token" could mean literal tokens, per-class rules or a TLD
  list. The criteria check only captured fragments, so a hard-coded list of
  strings would pass.
- **`<digits>)` escape precedes its capture** (flagged by: scope, clarity,
  completeness) — the `)` escape is unconditional, while the capture only
  records whether Linear rewrites it. Dotted tokens, by contrast, wait for the
  capture.
- **Capture is a prerequisite with no criterion or ordering** (flagged by:
  testability, dependency, scope) — the extended capture defines the
  dotted-token rule. However, no criterion requires it, and no dependency
  says it must come first.

### Findings

#### Major

- 🟡 **Clarity + Scope**: Digit-escape rule is broader than the defect and
  its acceptance criterion
  **Location**: Requirements: Required behaviour
  The rule escapes any continuation line starting `<digits>.`/`<digits>)`,
  while Defect 1 and the only criterion cover a lone number inside a list
  item. Two implementers could pick different triggers and both claim
  compliance.
- 🟡 **Clarity + Testability**: Dotted-token rule is ambiguous between a class
  rule and a recorded-token list
  **Location**: Requirements: Required behaviour / Assumptions
  "In no other token" reads as literal captured tokens, which contradicts the
  assumption that classes generalise via a TLD list. No criterion checks
  tokens held out of the capture, such as `lib.rs` or `config.yaml`.
- 🟡 **Dependency**: Linear's description parser and linkifier not named as
  an external dependency
  **Location**: Dependencies
  The whole rule set tracks Linear's API-side parsing, yet Dependencies lists
  only work items. PP-880 named the equivalent external coupling explicitly.
- 🟡 **Dependency**: Credentialed Linear workspace access not captured
  **Location**: Dependencies
  The capture extension, the read-back criterion and post-merge verification
  all need live access to the PP workspace. Post-merge verification also
  depends on the live state of PP-180, PP-733, PP-860 and PP-870.
- 🟡 **Testability**: Capture extension has no criterion, so the dotted-token
  criterion can pass vacuously
  **Location**: Requirements: Capture / Acceptance Criteria
  If the capture is never extended, the criterion passes on `R12` alone, and
  "in no other token" (`foo.toml`, `v1.2.3`, `e.g.`) is never exercised.
- 🟡 **Testability**: No negative criterion for genuine ordered lists and
  other digit lines
  **Location**: Acceptance Criteria
  An escaper that turns real ordered lists into paragraphs would pass every
  escaper criterion. Only the generic equivalence check might catch it, and
  only if the capture happens to include a real list.

#### Minor

- 🔵 **Scope + Clarity + Completeness**: `<digits>)` escape is required
  before the capture confirms it
  **Location**: Requirements: Capture / Required behaviour
  Either make it depend on the capture, as the dotted-token rule does, or
  state that it is a deliberate precaution because CommonMark treats `)` like
  `.`.
- 🔵 **Scope**: Parent epic still lists this item with three rewrites
  **Location**: Frontmatter: parent
  The PP-167 Stories entry for 0320 says "three meaning-changing rewrites;
  blocked by 0296". Both are now stale.
- 🔵 **Clarity**: Unclear which dot is escaped in multi-dot or trailing-dot
  tokens
  **Location**: Requirements: Required behaviour
  `a.b.c`, `v1.2.3`, `e.g.` and `jql.rs:167` leave open whether to escape
  every dot, the first dot or the dot before the suffix, and where the token
  ends.
- 🔵 **Clarity**: Passive "is only escaped" hides that Linear, not push,
  adds the escape
  **Location**: Requirements: Defect 1
  The bullet reads as required push behaviour.
- 🔵 **Clarity**: Actual result `160.` against Expected `0160.` reads like a
  typo
  **Location**: Requirements: Defect 1
  Say that Linear drops the leading zero.
- 🔵 **Dependency**: Order of work not stated
  **Location**: Requirements
  The capture must come before the dotted-token rule, and the escaper before
  the read-back.
- 🔵 **Dependency**: Downstream conflict clean-up not recorded under Blocks
  **Location**: Dependencies
  PP-880 defers 0161, 0203, 0276 and 0293 to this item. PP-906 names PP-180,
  PP-733, PP-860 and PP-870, and nothing reconciles the two lists.
- 🔵 **Testability**: Exclusions only partly covered by criteria
  **Location**: Acceptance Criteria
  Indented code, raw HTML and `mailto:` have no criterion.
- 🔵 **Testability**: Post-merge verification set is open-ended ("at least")
  **Location**: Acceptance Criteria: Post-merge verification
  The item doesn't say how to enumerate the full set at verification time.
- 🔵 **Completeness**: Open Questions says "None" while two assumptions are
  unverified
  **Location**: Open Questions
  Nothing records what happens if the capture refutes invisible escapes or
  TLD-based linking.
- 🔵 **Completeness**: Context does not state the current impact on sync
  **Location**: Context
  Affected items sit in conflict, and their Linear descriptions render
  wrongly.

#### Suggestions

- 🔵 **Scope**: Defect 1 could ship before the capture-gated Defect 2
  **Location**: Requirements
  Order the plan so the escaper and the digit rule land first.
- 🔵 **Dependency**: Coupling to PP-880's pinned CommonMark parser and digest
  recipe not named
  **Location**: Technical Notes
- 🔵 **Testability**: Rendering assumption is checked through API read-back,
  not the Linear UI
  **Location**: Assumptions
- 🔵 **Clarity**: "Escaper" and "recording tracker" used without
  introduction
  **Location**: Acceptance Criteria
- 🔵 **Clarity**: Ambiguous "which" in the pull-serialisation note
  **Location**: Technical Notes

### Strengths

- ✅ Both defects carry full reproductions: real items, exact input,
  expected and actual output, and capture fragment IDs (`X09`, `R12`).
- ✅ The two defects share one cause and one fix, so bundling them is
  justified. Out of scope is explicit, and bulk repair is excluded.
- ✅ Acceptance criteria are split by verification lane (CI, pinned live
  capture, one-off manual), and PP-880's equivalence gives a mechanical
  oracle.
- ✅ The parsed-text requirement lists every exempt construct and is
  justified by the double-escape risk from pulled `0160\.`. Idempotence has
  its own criterion.
- ✅ Both push paths are named by operation and by command, and are covered
  against a recording tracker.
- ✅ Drafting Notes explain the dropped Defect 3, the decision to keep bare
  URLs linked, and the linkifier-matching trade-off.

### Recommended Changes

1. **Pin the digit-escape trigger and add negative criteria** (addresses:
   digit-escape rule is broader than the defect; no negative criterion for
   genuine ordered lists)
   State one trigger, for example "a continuation line inside a list item
   that holds only `<digits>.` or `<digits>)`", in Summary, Required
   behaviour and the criterion. Add criteria that a real ordered list
   (`1. a` / `2. b`), a top-level lone number and `0160. more text` are each
   sent unchanged.
2. **Define the dotted-token matching rule as a property** (addresses:
   dotted-token rule is ambiguous; which dot is escaped)
   For example: "a bare token whose final dot-label, ignoring any
   `:<digits>` suffix, is a TLD Linear links, per the capture; every `.` in
   that token is escaped." Add a criterion with held-out tokens of each class
   (`lib.rs`, `config.yaml`, `README.md`).
3. **Add a criterion for the capture itself** (addresses: capture extension
   has no criterion; `<digits>)` escape before capture)
   Require the committed capture to contain each named token class and the
   `<digits>)` fragment, each annotated in the README. Then either make the
   `)` escape conditional on that result or state that it is pre-emptive.
4. **Fill out Dependencies** (addresses: Linear parser not named; workspace
   access; order of work; downstream clean-up)
   Add external dependencies on Linear's parser and linkifier and on
   credentialed PP workspace access. Add the ordering capture → dotted-token
   rule → escaper → read-back. Add under Blocks the clearing of PP-880's
   deferred conflicts, reconciling 0161, 0203, 0276 and 0293 with
   PP-180, PP-733, PP-860 and PP-870.
5. **Close the exclusion and verification gaps** (addresses: exclusions only
   partly covered; post-merge set open-ended)
   Add indented code, raw HTML and `mailto:` to the criteria. Fix the
   post-merge set to a named list or a named enumeration method.
6. **Tidy wording** (addresses: passive "is only escaped"; `160.` vs
   `0160.`; Open Questions vs Assumptions; Context impact; parent epic stale)
   Name Linear as the actor, and note the dropped zero. Say what happens if
   an assumption is refuted, and state the current conflict impact. Update
   PP-167's entry to two rewrites with no blocker.

## Per-Lens Results

### Clarity

**Summary**: PP-906 is tightly written. Its defects have concrete
reproductions, expected and actual results, and named capture fragments,
and its terms are anchored by links to the capture README and PP-880. The
main clarity risks are in Required behaviour. The digit-escape rule is
broader than the defect and its criterion, the dotted-token rule does not say
whether it is a class rule or a recorded-token list, and a few passive or
undefined phrases blur the actor or the exact character.

**Strengths**:
- Each defect follows Reproduction / Expected / Actual, naming real items and
  fragment IDs.
- Summary, Context and Requirements agree on the two rewrites. Out of scope
  excludes the cosmetic rewrites and the dropped code-span indentation.
- The parsed-text requirement lists every exempt construct, and the Technical
  Notes explain why.
- The push paths are named both by operation (`create`/`update`) and by
  command (`work create --push`/`sync`).
- Drafting Notes explain why bare URLs stay linked and why escaping tracks
  Linear's linkifier.

**Findings**:
- 🟡 major (high) — Requirements: Required behaviour — Digit-escape rule is
  broader than the defect and its acceptance criterion. "At the start of a
  continuation line" can be read either as every digit-led continuation line
  or as only a lone number in a list item. Only the narrow case is pinned.
- 🟡 major (medium) — Requirements: Required behaviour — Dotted-token rule is
  ambiguous between a class rule and a recorded-token list. "In no other
  token" contradicts the generalisation assumption. State what the rule is
  defined over.
- 🔵 minor (medium) — Requirements: Required behaviour — Unclear which dot is
  escaped in multi-dot or trailing-dot tokens, and whether `:167` belongs to
  the token.
- 🔵 minor (medium) — Requirements: Defect 1 — Passive "is only escaped"
  hides that Linear, not push, adds the escape.
- 🔵 minor (low) — Requirements: Capture / Required behaviour — The
  `<digits>)` escape is unconditional although the capture has not yet
  confirmed the rewrite.
- 🔵 minor (medium) — Requirements: Defect 1 — Actual `160.` against
  Expected `0160.` reads like a typo. Note the dropped leading zero.
- 🔵 suggestion (medium) — Acceptance Criteria — "Escaper" and "recording
  tracker" are used without introduction.
- 🔵 suggestion (low) — Technical Notes — Ambiguous "which" in the
  pull-serialisation note.

### Completeness

**Summary**: A very complete bug work item. Every expected section is present
and filled in, the frontmatter is intact, and each defect has a full
reproduction unit linked to a capture fragment. The gaps are small: unverified
assumptions sit next to "Open Questions: None", and Context does not state the
current impact.

**Strengths**:
- Full reproduction units for both defects.
- Context explains why the fix belongs on the push side.
- Acceptance criteria are layered by verification method.
- Out of scope is explicit, and Drafting Notes record rejected framings.
- Dependencies, Assumptions and the frontmatter are real and consistent.

**Findings**:
- 🔵 minor (medium) — Open Questions — "None" while two assumptions are
  unverified. Record what happens if the capture refutes them.
- 🔵 minor (low) — Context — Does not state the current impact: affected
  items stay conflicted, and Linear renders them wrongly.
- 🔵 suggestion (low) — Requirements — The `<digits>)` variant has no
  observed reproduction. Mark it as a precaution, or make it conditional.

### Dependency

**Summary**: The one upstream work-item blocker, PP-880, is named and marked
done. Dependencies omits the couplings the body depends on: Linear's
API-side parser and linkifier, and credentialed access to the PP Linear
workspace. The order of work inside the item is also left implicit, which
PP-880 itself made explicit.

**Strengths**:
- The PP-880 dependency is stated with its status, matching PP-880's
  "Blocks: 0320".
- The parent and related items appear in both frontmatter and References.
- Out of scope excludes Jira, Shortcut and upstream reporting.
- Technical Notes acknowledge linkifier drift and how it surfaces.

**Findings**:
- 🟡 major (high) — Dependencies — Linear's description parser and linkifier
  are not named as an external dependency.
- 🟡 major (high) — Dependencies — Credentialed PP Linear workspace access
  for the capture, read-back and post-merge verification is not captured.
- 🔵 minor (medium) — Requirements — Ordering not stated: capture before the
  dotted-token rule, and the escaper before the read-back.
- 🔵 minor (medium) — Dependencies — Downstream conflict clean-up deferred by
  PP-880 (0161, 0203, 0276, 0293) is not recorded under Blocks or reconciled
  with PP-180, PP-733, PP-860 and PP-870.
- 🔵 suggestion (low) — Technical Notes — Coupling to PP-880's pinned
  CommonMark parser and digest recipe is not named.

### Scope

**Summary**: A coherent, well-bounded bug. Both defects come from one cause
and share one fix used on both push paths. The scope problems are small: the
continuation-line rule is broader than the defect, the `)` variant is
required before the capture confirms it, the parent epic is stale, and
Defect 1 could ship ahead of the capture-gated Defect 2.

**Strengths**:
- One cause and one fix justify bundling the two defects.
- Out of scope draws clear lines.
- Repair is limited to the next push, with no bulk rewrite.
- The dropped Defect 3 is recorded.
- The bug kind fits the size of the work.

**Findings**:
- 🔵 minor (medium) — Requirements — The continuation-line escape rule is
  broader than the observed defect.
- 🔵 minor (medium) — Requirements — The `<digits>)` escape is required
  before the capture confirms it.
- 🔵 minor (high) — Frontmatter: parent — PP-167 still lists this item as
  fixing three rewrites, blocked by 0296.
- 🔵 suggestion (medium) — Requirements — Defect 1 could ship before the
  capture-gated Defect 2. Order the plan accordingly.

### Testability

**Summary**: Highly testable. The reproductions are exact, the criteria are
split into three lanes, and most are Given/When/Then with a definitive
outcome. The gaps are at the edges: the capture has no criterion of its own,
nothing tests tokens outside the capture, nothing protects genuine ordered
lists, and not every exclusion is covered.

**Strengths**:
- Complete reproductions with fragment IDs.
- Distinct verification lanes.
- PP-880 equivalence gives a mechanical oracle.
- Idempotence on already-escaped input has its own criterion.
- Both push paths are covered against a recording tracker.

**Findings**:
- 🟡 major (high) — Requirements: Capture / Acceptance Criteria — The
  capture extension has no criterion, so the dotted-token criterion can pass
  vacuously.
- 🟡 major (medium) — Requirements: Required behaviour / Assumptions — No
  expected outcome for dotted tokens outside the capture. A hard-coded
  escaper would pass.
- 🟡 major (medium) — Acceptance Criteria — No negative criterion that
  genuine ordered lists and other digit lines are left alone.
- 🔵 minor (high) — Acceptance Criteria — Indented code, raw HTML and
  `mailto:` exclusions have no criterion.
- 🔵 minor (medium) — Acceptance Criteria: Post-merge verification — The
  "at least" set is open-ended, with no enumeration method.
- 🔵 suggestion (low) — Assumptions — Rendering invisibility is checked
  through API read-back, not the Linear UI.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-11-23T16:01:45+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity + Scope**: Digit-escape rule broader than the defect — Resolved
- 🟡 **Clarity + Testability**: Dotted-token rule ambiguous — Partially
  resolved (rule now a TLD property; token boundaries still undefined)
- 🟡 **Dependency**: Linear parser and linkifier not named — Resolved
- 🟡 **Dependency**: Workspace access not captured — Resolved
- 🟡 **Testability**: Capture extension has no criterion — Resolved
- 🟡 **Testability**: No negative criterion for real ordered lists — Resolved
- 🔵 **Scope + Clarity + Completeness**: `<digits>)` escape before capture —
  Resolved
- 🔵 **Scope**: Parent epic stale — Resolved
- 🔵 **Clarity**: Which dot is escaped — Resolved
- 🔵 **Clarity**: Passive "is only escaped" — Resolved
- 🔵 **Clarity**: `160.` vs `0160.` — Resolved
- 🔵 **Dependency**: Order of work not stated — Partially resolved (order
  now contradicts the first assumption)
- 🔵 **Dependency**: Downstream clean-up not under Blocks — Resolved
- 🔵 **Testability**: Exclusions partly covered — Resolved
- 🔵 **Testability**: Post-merge set open-ended — Resolved
- 🔵 **Completeness**: Open Questions "None" beside unverified assumptions —
  Still present (downgraded to suggestion)
- 🔵 **Completeness**: Context lacks impact — Resolved
- 🔵 **Testability**: Rendering checked via API, not UI — Still present
  (now major; see below)

### New Issues Introduced

- 🟡 **Testability + Dependency**: The invisibility gate can neither observe
  rendering nor run in time. The API read-back returns stored markdown, and
  Linear already stores `0160\.` (`R11`), so it cannot show what a reader
  sees. Dependencies also runs the read-back only after the escaper lands,
  while the first assumption gates the escaper on it.
- 🟡 **Clarity + Testability**: The post-merge "shows no backslash" check
  doesn't say which surface to inspect. Stored markdown is expected to
  contain `crates\.io`, so the check passes only on the rendered page.
- 🟡 **Clarity + Testability**: Dotted-token boundaries are undefined:
  sentence-final `crates.io.`, `(crates.io)`, `src/client.rs`, `a@b.io`.
- 🔵 **Testability**: "Any container" for digits is not pinned by any
  criterion for blockquotes, nested items or ordered-list items.
- 🔵 **Clarity**: The top-level lone-number escape falls outside the stated
  rationale.
- 🔵 **Clarity**: "The capture confirms…" is stated as fact while the
  capture is still pending.
- 🔵 **Testability**: The committed escaped fragments can drift from the
  escaper's output.
- 🔵 **Dependency**: Push paths from 0230 (promotion, pending-push
  adoption) and 0290's changes to the push path are not coordinated.
- 🔵 **Dependency**: Frontmatter lacks `blocked_by` PP-880 and the Blocks
  relations.
- 🔵 **Scope**: The digit rule could ship without the capture-gated token
  rule; the capture is in effect a spike inside the bug.
- 🔵 **Clarity**: The post-merge criterion is hard to parse and leaves the
  resolving command implicit.
- 🔵 **Completeness**: The Summary states the defect but not the fix.
- 🔵 Suggestions: angle-bracket autolinks; "the fixture" referent;
  continuation lines decided by CommonMark, with indentation allowed; a
  Shortcut follow-up and 0321 as Related; post-merge clean-up limited to
  the four named items.

### Assessment

All six pass-one majors are resolved or narrowed, but the edits introduced
two new majors (the post-merge surface and the read-back ordering), and the
token-boundary gap remains. These are small, local fixes: name the rendered
UI as the surface for invisibility, push hand-escaped fragments before the
escaper lands, and define the delimiters that end a token. After that, the
item should be ready for planning.

## Re-Review (Pass 3) — 2026-11-23T16:12:57+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability + Dependency**: Invisibility gate unobservable and
  mis-ordered — Resolved
- 🟡 **Clarity + Testability**: Post-merge surface unnamed — Resolved
- 🟡 **Clarity + Testability**: Dotted-token boundaries undefined —
  Resolved, but introduced the bare-URL conflict below
- 🔵 **Testability**: Digit containers unpinned — Resolved
- 🔵 **Clarity**: Top-level rationale — Resolved
- 🔵 **Clarity**: "The capture confirms…" — Resolved
- 🔵 **Testability**: Escaped fragments can drift — Resolved
- 🔵 **Dependency**: PP-760 and PP-876 paths — Resolved on this side;
  PP-876 lacks the back-reference
- 🔵 **Dependency**: Frontmatter relations — Partially resolved;
  `blocked_by` now contradicts the body
- 🔵 **Scope**: Digit rule shippable alone; capture as spike — Partially
  resolved (one item kept by decision; scope still suggests a split)
- 🔵 **Clarity**: Post-merge criterion hard to parse — Resolved
- 🔵 **Completeness**: Summary lacks the fix — Resolved
- 🔵 **Completeness**: Open Questions "None" — Resolved

### New Issues Introduced

- 🟡 **Clarity**: The bare-URL exemption conflicts with the token boundary
  rule. `crates.io` inside `https://crates.io/x` is bounded by `/`, and
  the `a@b.io` criterion contradicts "`mailto:` sent unchanged". The item
  never says how a bare scheme URL is recognised or where it ends.
- 🟡 **Testability**: No criterion checks that the local file and baseline
  stay unescaped after a push; promotion and adoption both write the local
  file.
- 🔵 **Completeness + Dependency + Clarity**: `blocked_by` PP-880
  contradicts "Blocked by: none".
- 🔵 **Clarity**: "The capture" names three artefacts; "body" and
  "fragment" are used interchangeably.
- 🔵 **Clarity**: "The escaper" is used both with and without the
  dotted-token rule.
- 🔵 **Clarity**: "Repaired by the next push" hides the manual
  `--resolve` step.
- 🔵 **Completeness**: `server.pid` and `jql.rs:167` outcomes are cited
  to `R12`, which records only `crates.io`.
- 🔵 **Testability**: Raw `linear update --body-file` and non-Linear
  trackers have no verbatim criterion; case sensitivity of the TLD match is
  unspecified.
- 🔵 **Dependency**: PP-876 and PP-907 lack back-references, and nothing
  says whether this item waits for PP-907.
- 🔵 **Scope**: Post-merge clean-up sits in this item's done criteria, yet
  Dependencies lists it under Blocks.
- 🔵 Suggestions: "stores as plain text" should read "renders"; define
  "text run"; partly escaped tokens (`a.b\.io`); digit length beyond
  nine; redaction allowlist for new read-backs; a note on the pre-emptive
  cases.

### Assessment

The pass-two majors are resolved. Two new majors remain: one follows
directly from the boundary rule (bare URLs), and one is a missing guard
(local file unchanged). Both are small to fix. The rest is terminology and
bookkeeping. One more round of edits should bring the item to COMMENT or
APPROVE.

## Re-Review (Pass 4) — 2026-11-23T16:31:32+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: Bare-URL exemption against the boundary rule — Resolved
- 🟡 **Testability**: Local file unchanged unverified — Partially resolved
  (the file clause holds; the baseline clause cannot fail, see below)
- 🔵 **Completeness + Dependency + Clarity**: `blocked_by` contradiction —
  Resolved
- 🔵 **Clarity**: "The capture" overloaded — Resolved (one residual
  ambiguity, see below)
- 🔵 **Clarity**: "The escaper" overloaded — Resolved
- 🔵 **Clarity**: Repair trigger hidden — Resolved
- 🔵 **Completeness**: `R12` citation — Resolved
- 🔵 **Testability**: Raw update, non-Linear trackers, TLD case — Resolved
- 🔵 **Dependency**: PP-876 and PP-907 back-references — Resolved
- 🔵 **Scope**: Post-merge clean-up vs Blocks — Resolved; scope still
  suggests limiting the set to the four named items
- 🔵 **Scope**: Split and spike — Still raised; kept as one item by
  decision

### New Issues Introduced

- 🟡 **Clarity**: The byte-identical criterion compares against "its fragment
  in the escaped capture" without naming `escaped/pushed.md` or the
  read-back; only one of them can pass.
- 🔵 **Testability + Clarity**: "The baseline records the unescaped local
  body" cannot fail, since the baseline stores a normalised digest.
- 🔵 **Testability**: No criterion for a real list interrupting a paragraph
  (`1. text`), or for `0160) more text`.
- 🔵 **Testability**: Dotted tokens are verified only in plain text, not in
  bold, headings or table cells.
- 🔵 **Testability**: Post-merge `synced` can fail for reasons outside this
  item.
- 🔵 **Clarity**: Dotted-token capture classes overlap (`server.pid` vs
  `foo.md`).
- 🔵 **Clarity**: Sync terms used without a pointer (baseline, promotion,
  adoption, pinned parser, the 0320 alias).
- 🔵 **Clarity**: Order of hand-escaping is unclear, and nothing says what
  happens to the escaped capture if only the digit rule lands.
- 🔵 **Dependency**: PP-885's shared Markdown tooling is recorded only on
  PP-885's side.
- 🔵 Suggestions: the order between the two captures; keep the escaped
  issue until the UI check is recorded; both landing orders with PP-876;
  References missing PP-760, PP-876, PP-907 and the allowlist; scope the
  read-back equivalence requirement to what is verified.

### Assessment

The item is ready for planning: one major remains, below the REVISE
threshold of two, and it is a one-word fix (name `escaped/pushed.md`). The
remaining minors are terminology and edge-case criteria that planning can
absorb or that a short tidy-up would close.

## Verdict Change — 2026-11-23T16:43:46+00:00

**Verdict:** APPROVE

Changed from COMMENT by the reviewer after a tidy-up that resolved the
remaining major (the byte-identical criterion now names
`escaped/pushed.md`) and most of the pass-4 minors. The tidy-up was not
re-reviewed by the lenses.
