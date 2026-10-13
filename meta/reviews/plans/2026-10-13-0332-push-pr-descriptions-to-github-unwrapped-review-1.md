---
type: "plan-review"
id: "2026-10-13-0332-push-pr-descriptions-to-github-unwrapped-review-1"
title: "Plan Review: Push PR Descriptions to GitHub Unwrapped Implementation Plan"
date: "2026-10-13T22:47:15+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-13-0332-push-pr-descriptions-to-github-unwrapped"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "standards", "documentation"]
review_number: 1
review_pass: 2
tags: ["collaboration", "github", "markdown", "pulldown-cmark", "describe-pr"]
last_updated: "2026-10-13T23:26:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Push PR Descriptions to GitHub Unwrapped Implementation Plan

**Verdict:** REVISE

The plan is structurally sound. The parser lives in a pure, pinned technical
library. The domain reaches it through an intent-named port, as ADR-0069
requires. The library-crate checklist is covered in full, and every cited
path, line and helper checks out bar one.

One defect blocks implementation. The span rule produces a reversed span when
a `SoftBreak` is the last child of a link or image, and that duplicates the
document. A probe against `pulldown-cmark` 0.13.4 confirmed it.

Three gaps are cheaper to fix:

- a config-independence test that cannot fail
- no CHANGELOG entry for the contract change
- a triage rule that lets GitHub-structure regressions ship behind a follow-up

### Cross-Cutting Themes

- **Span rule edge cases outside the fixture table** (flagged by: correctness,
  test-coverage, compatibility). These inputs are all unexercised:
  - links and images that wrap before `]`
  - non-ASCII text next to a break
  - trailing tabs and form feeds
  - paragraphs of three or more lines
  - constructs where `pulldown-cmark` and cmark-gfm diverge

  The event-stream check validates the parser against itself, not against
  GitHub. Edge coverage therefore has to come from fixtures and probes.
- **The `pup.ron` rule comment omits `std::fs`/`tokio`** (flagged by:
  code-quality, standards, documentation). The comment says "no process
  spawning, no transport", but the deny list is wider.
- **Adapter naming** (flagged by: code-quality, standards).
  `GitHubFlavouredMarkdown` uses UK spelling of a US proper noun and
  names the dialect, not the role. Its siblings use `GitHub<Role>`.
- **Use case parameter growth** (flagged by: architecture, code-quality).
  `update_pull_request_body` grows to nine positional `&dyn` parameters under
  `#[allow(clippy::too_many_arguments)]`.
- **Stale shared-probe comment** (flagged by: standards, documentation).
  `test_import_rule.py:916-920` lists the crates the shared probe serves, and
  `markdown` is not added.

### Tradeoff Analysis

- **Shipping the unwrap vs structural fidelity on GitHub**: compatibility wants
  any HTML-check failure to block Phase 3. The plan, and the work item's
  no-opt-out stance, favour shipping with follow-ups. The recommendation is
  to block on divergences that break structure (lost tables, alerts or lists).
  Divergences that only leave lines wrapped can still go to a follow-up,
  because they are no worse than today.
- **Port simplicity vs pairing safety**: architecture notes that the
  preparation port and the PATCH port are injected independently. A forge
  bundle would tie them together but widens the scope. With one forge, a
  doc note is enough.

### Findings

#### Major

- 🟡 **Correctness**: SoftBreak followed by End(Link/Image) yields a reversed
  span that duplicates the document
  **Location**: Phase 1, Section 4: Implementation
  For `[a\n](u)`, the event after the `SoftBreak` (range `2..3`) is
  `End(Link)` with range `0..7`, so `continuation_start` returns `0`. The
  span `2..0` makes `unwrap_soft_line_breaks` emit `[a [a\n](u)`. The
  orchestrator reproduced this against 0.13.4, and `![a\n](u)` behaves the
  same way.
- 🟡 **Test Coverage**: Config-independence test only checks the two runs
  agree, so it passes when both are wrong
  **Location**: Phase 3, Section 3:
  `update_body_ignores_repository_line_width_conventions`
  The test compares the two runs only with each other. Its lines are all under
  40 columns, so it passes even if the unwrap is skipped or the body is
  re-wrapped to 40 columns.
- 🟡 **Compatibility**: Known structure-breaking parser divergences ship with
  only a follow-up
  **Location**: Phase 2, Section 3: Triage
  An HTML-check failure means the unwrap changed GitHub's block structure. The
  plan files a follow-up and then switches the behaviour on unconditionally
  with no opt-out.
- 🟡 **Documentation**: No CHANGELOG entry for a user-visible change that
  replaces the byte-for-byte body contract
  **Location**: Migration Notes / Phase 3
  `CHANGELOG.md` keeps a detailed `[Unreleased]` section, and no phase adds a
  `### Changed` bullet to it.

#### Minor

- 🔵 **Correctness**: Trailing-whitespace class differs from the parser's, and
  tabs count toward hard breaks
  **Location**: Phase 1, Section 4 (`trailing_whitespace_start`)
  The parser trims space, tab, VT and FF, but the span rule trims only space
  and tab. `pulldown-cmark` also treats `\t\t` as a hard break, where cmark-gfm
  needs two spaces.
- 🔵 **Test Coverage**: Unchanged-file test should use a body the unwrap would
  change
  **Location**: Phase 3, Section 3: `update_body_leaves_the_body_file_unchanged`
  With a body that has no soft breaks, a regression that wrote back to the file
  would go undetected.
- 🔵 **Test Coverage**: Testing Strategy claims a test for preparing only after
  resolution, but none is planned
  **Location**: Testing Strategy: Unit Tests
  No planned test fails if `prepare_for_publishing` runs before resolution or
  on the failure path.
- 🔵 **Test Coverage**: No fixture puts non-ASCII text next to a soft break
  **Location**: Phase 1, Section 3
  Every fixture is ASCII, while the prose in `meta/prs/` routinely uses em
  dashes next to line endings.
- 🔵 **Test Coverage**: Edge cases missing for trailing tabs, tab indentation
  and several breaks in one paragraph
  **Location**: Phase 1, Section 3
  Removing `b'\t'` from either `matches!` would survive the suite. So would an
  off-by-one between adjacent spans.
- 🔵 **Compatibility**: Verification cannot detect lines left wrapped where
  cmark-gfm sees a soft break
  **Location**: Phase 2, Section 2
  The `mode=markdown` comparison finds only over-unwrapping. It cannot find
  lines left wrapped.
- 🔵 **Compatibility**: No fixtures for known pulldown-cmark vs cmark-gfm edge
  constructs
  **Location**: Phase 1, Section 3
  The fixtures do not cover lowercase or unknown alert kinds, non-pipe lines
  after a table, footnote lazy continuation, or `$` math.
- 🔵 **Compatibility**: Plan and research disagree on whether
  `pulldown-cmark-escape` enters the lockfile
  **Location**: Key Discoveries; Phase 3, Section 5
  The research lists two new packages, but the plan expects one.
- 🔵 **Architecture**: Body preparation and body transport are independent
  ports that must be paired by convention
  **Location**: Phase 3, Sections 1-3
  Nothing in the types ties the preparer to the forge that receives the body.
- 🔵 **Architecture / Code Quality**: `update_pull_request_body` grows to nine
  parameters under a clippy allow
  **Location**: Phase 3, Section 1
  The new domain test also repeats the full nine-argument call, because the
  `update(...)` helper hard-codes the publishing fake.
- 🔵 **Architecture**: A generically named `markdown` library has a
  GitHub-specific dialect baked in, which limits the planned 0320 reuse
  **Location**: Phase 1, Section 4; What We're NOT Doing
  0320's Linear defects include wrapped code spans, which this function
  deliberately leaves alone.
- 🔵 **Code Quality**: The `min(next_event)` invariant relies on a test case
  buried inside a multi-case test
  **Location**: Phase 1, Section 4
  The deep-indent case is the fourth of four cases in a test whose name does
  not point at the clamp.
- 🔵 **Code Quality**: Shared assertion bundles many checks without saying
  which one failed
  **Location**: Phase 1, Section 3
  `assert_unwraps` makes up to six assertions per fixture, and the plan
  specifies neither failure messages nor `#[track_caller]`.
- 🔵 **Code Quality / Standards / Documentation**: The `pup.ron` rule comment
  does not match its own deny list
  **Location**: Phase 1, Section 5
  The comment does not justify the `std::fs` and `tokio` bans.
- 🔵 **Code Quality / Standards**: Adapter type misspells a proper noun and
  departs from the `GitHub<Role>` naming of sibling adapters
  **Location**: Phase 3, Section 2
  `GitHubFlavouredMarkdown` uses a spelling not found anywhere else in the
  repository, and names the dialect rather than the role.
- 🔵 **Standards**: `review_fixtures_join` is named for where the fixtures came
  from, not what they test
  **Location**: Phase 1, Section 3
  The name references the review that produced the inputs.
- 🔵 **Standards / Documentation**: The shared-probe block comment lists its
  rules and is not updated
  **Location**: Phase 1, Section 5
  `test_import_rule.py:916-920` lists remote-projection and tracker-support
  only.
- 🔵 **Documentation**: SKILL.md exit-code citation is off by one line
  **Location**: Current State Analysis
  The Exit 1 bullet is at `SKILL.md:180-181`, not 181-182.
- 🔵 **Documentation**: Missing checklist obligation is found but not
  documented in tasks/README.md
  **Location**: Key Discoveries / Phase 1, Section 5
  The README's library-crate checklist still omits the `pup.ron:443`
  alternation.
- 🔵 **Documentation**: Existing doc comments not updated for the new
  publishing step
  **Location**: Phase 3, Sections 1-2
  The plan leaves the `update_pull_request_body` doc unchanged and gives no
  wording for the `github` crate doc.
- 🔵 **Documentation**: Assumes describe-pr will copy the corpus record into
  the PR description
  **Location**: Phase 2, Section 4
  Nothing in `describe-pr` reads plan records, yet the work item's last
  acceptance criterion depends on it.

#### Suggestions

- 🔵 **Architecture**: The infallible preparation port cannot later refuse a
  body. Note that refusal would be a port change.
- 🔵 **Code Quality**: Parser options are widened to crate visibility mainly
  for the tests. The scratch verifier copies the five flags by hand.
- 🔵 **Standards**: `pub(crate)` on `github_extensions` is wider than the
  stated reason needs. A child `tests` module already sees private items.
- 🔵 **Standards**: The plan says the line-443 alternation lists every crate,
  but it does not. It omits `config`, `config_adapters` and
  `graphql_test_support`.
- 🔵 **Standards**: Consider a `deny.toml` wrapper ban,
  `{ crate = "pulldown-cmark", wrappers = ["markdown"] }`, following the
  `serde-saphyr` precedent.
- 🔵 **Test Coverage**: The new route-setup helper duplicates the existing
  `resolving_server()`. Build on it instead.
- 🔵 **Test Coverage**: Consider the existing golden `cli_surface.rs` pattern
  for the `--help` pin.
- 🔵 **Compatibility**: The Migration Notes do not cover future `update-body`
  callers or trailer lines that rely on single newlines.
- 🔵 **Documentation**: The added `collaboration.md` sentence is long, and it
  does not mention that `>` markers and indentation go with the break.

### Strengths

- ✅ Dependency direction follows ADR-0069. The domain gains only a trait, the
  adapter takes the adapter → technical-library edge, and the composition root
  binds them.
- ✅ The functional core is pure. A `cargo-pup` rule enforces this, and a
  probe pair tests the rule.
- ✅ The public surface is one function with no `pulldown_cmark` type, pinned
  through `public-api.txt`.
- ✅ The exact pin and characterisation test turn a parser bump into a precise
  failure. The pin follows the `similar = "=2.7.0"` precedent.
- ✅ The six-point library-crate checklist is covered in full, including the
  `pup.ron:443` alternation the README omits.
- ✅ Lockfile claims check out: `bitflags` 2.13.1, `memchr` 2.8.3 and `unicase`
  2.9.0 are already present. MSRV and licence are compatible.
- ✅ Most of the span rule is right. Every Phase 1 expected output follows from
  the algorithm under LF and CRLF, slices fall on UTF-8 boundaries, and a
  `SoftBreak` is never the final event.
- ✅ The phases merge on their own and rollback is limited to Phase 3.
- ✅ The plan fixes the existing SKILL.md exit-code error and correctly leaves
  the generated reference docs alone.
- ✅ Constructs that need no special case fall out of the event stream rather
  than being hand-coded.

### Recommended Changes

1. **Clamp the continuation start to the line ending** (addresses: SoftBreak
   followed by End(Link/Image)). Make `continuation_start` fall back to the
   prefix scan when `next_event < line_start`. Add red-first fixtures
   `[a\n](u)` → `[a ](u)` and `![a\n](u)` → `![a ](u)`, and add the same case
   to the characterisation test.
2. **Make the config-independence test fail on a real regression** (addresses:
   Config-independence test only checks the two runs agree). Use a paragraph
   longer than 40 columns once unwrapped. Assert each run's PATCHed body
   equals the literal expected string, and that both runs exit 0.
3. **Make structure-breaking divergences block Phase 3** (addresses: Known
   structure-breaking parser divergences; Verification cannot detect lines
   left wrapped; No fixtures for known divergent constructs).
   - Split the triage row in two:
     - HTML differs → blocking; fix the span rule or skip the construct
     - line left wrapped → follow-up
   - Add a `mode=gfm` `<br>` count to detect lines left wrapped.
   - Add hand-written probes for the divergent constructs.
4. **Add a CHANGELOG `### Changed` entry in Phase 3** (addresses: No CHANGELOG
   entry). Say what is unwrapped and that the files stay wrapped. Include the
   hard-break guidance and the exit-code correction.
5. **Widen the fixture table** (addresses: non-ASCII, trailing tabs and
   multi-break paragraphs; Trailing-whitespace class; buried `min` invariant;
   `review_fixtures_join`).
   - Add these fixtures:
     - `café —\nnaïve`
     - `foo\t\nbar`
     - `- item\n\tcontinued`
     - `one\ntwo\nthree\nfour`, plus a blockquote variant
   - Give the deep-indent case its own test, named for the invariant.
   - Rename `review_fixtures_join` after the behaviour it tests.
   - Take the span start from the preceding event's end, or match VT/FF too.
   - Record the tab-as-hard-break divergence.
6. **Tighten the Phase 3 tests** (addresses: unchanged-file test; claimed
   preparation-ordering test; helper duplication; `--help` pin).
   - Use a wrapped body in the unchanged-file test.
   - Either drop the ordering claim or make the failure-path test assert the
     publishing fake is never called.
   - Build the PATCH helper on `resolving_server()`.
   - Consider the golden `cli_surface.rs` pattern for `--help`.
7. **Fix the naming and comment drift** (addresses: adapter naming; `pup.ron`
   comment; shared-probe comment; pub(crate)).
   - Rename the adapter `GitHubPullRequestBodyPublishing`, or spell it
     `GitHubFlavoredMarkdown`.
   - Reword the `pup.ron` comment to cover the filesystem and runtime bans.
   - Generalise the comment at `test_import_rule.py:916-920`.
   - Make `github_extensions` private.
8. **Close the documentation gaps** (addresses: README checklist; doc comments;
   corpus record; citation; collaboration.md wording).
   - Add the `pup.ron:443` step to the README checklist and update its count.
   - Extend the `update_pull_request_body` and `github` crate docs.
   - Replace "describe-pr copies this record" with a Phase 3 manual check.
   - Fix the citation to `SKILL.md:180-181`.
   - Split the `collaboration.md` sentence and mention that `>` markers go with
     the break.
9. **Optional evolution notes** (addresses: parameter growth; port pairing;
   0320 reuse; `pulldown-cmark-escape`; `deny.toml` wrapper ban).
   - Consider letting the `update(...)` test helper take the publishing port.
   - Note in the trait doc that the preparer must match the body-update port.
   - State that 0320 will likely need a dialect-aware entry point.
   - Verify the lockfile diff after the sync.
   - Consider the `deny.toml` wrapper ban.

## Per-Lens Results

### Architecture

**Summary**: Architecturally, the plan holds up and fits ADR-0069: a pure
technical library behind a narrow, pinned surface, reached only through a
domain-owned port that the GitHub adapter implements and the composition root
wires. The remaining concerns are about evolution: the new port is wired
separately from the forge's body-update port, the use case's parameter list
keeps growing, and the 0320 reuse claim is weaker than it looks.

**Strengths**:
- Dependency direction follows ADR-0069 rules 1, 4 and 5.
- The functional core is pure, and `markdown_transforms_text_only` enforces it
  mechanically.
- The public surface is one pinned function, and the parser options are
  crate-private.
- A domain port beats hiding the unwrap in `OctocrabClient::update_body`.
- The phases merge on their own, and rollback is limited to Phase 3.
- The library-crate checklist is covered in full, including `pup.ron:443`.

**Findings**:
- 🔵 minor (medium) — Phase 3, Sections 1-3: Body preparation and body
  transport are independent ports that must be paired by convention. A second
  forge could be wired with `GitHubFlavouredMarkdown` beside a GitLab body
  update. Document the pairing, or have `github` expose the pair together.
- 🔵 minor (medium) — Phase 3, Section 1: `update_pull_request_body` grows to
  nine parameters under a clippy allow. Group the forge ports into one
  parameter, or record a follow-up.
- 🔵 minor (medium) — Phase 1, Section 4; What We're NOT Doing: A
  generically named `markdown` library has a GitHub-specific dialect baked in.
  0320 reuse will probably need a dialect parameter. Name it honestly, or
  record that expectation.
- 🔵 suggestion (low) — Phase 3, Section 1: The infallible preparation port
  cannot later refuse a body. YAGNI is reasonable, but note that refusal would
  be a port change.

### Code Quality

**Summary**: The design is simple. A small pure function with helpers named
for their intent, a single-purpose adapter behind an intent-named port, and one
pinned public symbol. The issues are small maintainability points: the
parameter list grows, the subtle invariant is guarded only by a hard-to-find
test, the shared assertion may give vague failures, and a few names and
comments drift from the code.

**Strengths**:
- A pure, infallible `&str -> String` function, with helpers named for the
  span rule.
- The GitHub-specific rule is isolated in its adapter, not folded into
  `OctocrabClient`.
- A characterisation test pins the parser's behaviour before anything builds
  on it.
- The end-to-end helpers remove duplication.
- Tests are named for behaviour, and the identity fake keeps the existing
  tests' meaning.
- The event stream leaves near-zero branching in the transform.

**Findings**:
- 🔵 minor (high) — Phase 3, Section 1: The use case grows to nine positional
  parameters. Group the forge collaborators, or let the test helper take the
  publishing port.
- 🔵 minor (medium) — Phase 1, Section 4: The `min(next_event)` invariant
  relies on a buried test case. Give it a dedicated test, and consider operand
  names like `container_prefix_end.min(continuation_content_start)`.
- 🔵 minor (medium) — Phase 1, Section 3: `assert_unwraps` does not say which
  check failed. Add messages per check and variant, plus `#[track_caller]`.
- 🔵 minor (high) — Phase 1, Section 5: The pup rule comment does not match its
  deny list (`std::fs`, `tokio`).
- 🔵 suggestion (medium) — Phase 3, Section 2: The adapter name re-spells a
  proper noun and names the dialect, not the role. Use
  `GitHubPullRequestBodyPublishing` or `GitHubFlavoredMarkdown`.
- 🔵 suggestion (low) — Phase 1, Section 4: Parser options are widened to
  crate visibility mainly for tests, and the scratch verifier copies the
  flags by hand.

### Test Coverage

**Summary**: Every acceptance criterion maps to a named test or a manual step,
and the test-first order is clear. The `markdown` unit suite is strong. Phase 3
is weaker: the config-independence test only compares two runs with each other,
the unchanged-file test does not specify a wrapped body, and the Testing
Strategy claims an ordering test that is not planned. A few cheap edge fixtures
are missing.

**Strengths**:
- Every 0332 acceptance criterion maps to a test or a manual step.
- `assert_unwraps` gives strong protection against mutations on every fixture.
- The characterisation test fails precisely on a parser bump.
- `MarkedPublishing` kills the raw-body mutation.
- Tests are spread sensibly across the unit, adapter, domain and end-to-end
  levels.
- Corpus failures become red-first fixtures.

**Findings**:
- 🟡 major (high) — Phase 3, Section 3: The config-independence test only
  checks that the two runs agree, so it passes when both are wrong. Use a
  paragraph longer than 40 columns, assert the literal expected body, and
  assert success.
- 🔵 minor (medium) — Phase 3, Section 3: The unchanged-file test should use a
  body the unwrap would change. Also assert success and that the PATCH was
  received.
- 🔵 minor (high) — Testing Strategy: The claimed ordering test is not
  planned. Drop the claim, or assert that the fake is not called on the
  failure path.
- 🔵 minor (medium) — Phase 1, Section 3: No non-ASCII fixture. Add
  `café —\nnaïve` and a blockquote variant.
- 🔵 minor (medium) — Phase 1, Section 3: Trailing-tab, tab-indent and
  multi-break fixtures are missing.
- 🔵 suggestion (high) — Phase 3, Section 3: `serve_pull_request_42`
  duplicates `resolving_server()`.
- 🔵 suggestion (medium) — Phase 3, Section 3: Use the golden
  `cli_surface.rs` pattern for the `--help` pin.

### Correctness

**Summary**: The span rule matches pulldown-cmark 0.13.4's own paragraph and
container logic for almost every fixture. Every Phase 1 expected output
follows from the algorithm under LF and CRLF. One case breaks it: a `SoftBreak`
that is the last child of a link or image is followed by `End(Link|Image)`,
whose range starts at `[`. That yields a reversed span that duplicates the
document. Phase 3 is sound.

**Strengths**:
- `min(next event, prefix scan)` handles both research counter-examples and
  deeper nested cases.
- All slice indices fall on UTF-8 boundaries.
- A `SoftBreak` is never the final event, and never comes directly before
  `End(Paragraph)`.
- CRLF and lone-CR line endings are handled.
- Every fixture's expected output follows from the algorithm.
- Phase 3's ordering and fakes are consistent with the design.

**Findings**:
- 🟡 major (high) — Phase 1, Section 4: A `SoftBreak` followed by
  `End(Link/Image)` yields a reversed span. `[a\n](u)` produces span `2..0`,
  and the output is `[a [a\n](u)`. The orchestrator confirmed this with a
  probe against 0.13.4, and `![a\n](u)` behaves the same way. Fall back to
  the prefix scan when `continuation.start < line_ending.end`, add red-first
  fixtures, and consider asserting that spans are non-decreasing.
- 🔵 minor (medium) — Phase 1, Section 4: The trailing-whitespace class (space
  and tab) differs from the parser's (which adds VT and FF). The parser also
  treats tabs as forming hard breaks, but cmark-gfm does not. Take the span
  start from the preceding event's end, and record the tab divergence.

### Compatibility

**Summary**: The dependency claims check out against `cli/Cargo.lock`. MSRV
and licence are fine, the exact pin follows the precedent, and the API change
has one caller. The main risk is that pulldown-cmark and cmark-gfm may disagree
on where paragraphs end. The verification catches only one direction of that,
and it lets structure-breaking divergences ship with a follow-up.

**Strengths**:
- Lockfile claims are verified, and no new deny skip is needed.
- The exact pin has a rationale and a characterisation test behind it.
- No parser types appear in the public surface.
- The `--help` surface is pinned before the change.
- The signature break is internal to a `publish = false` workspace.
- Every consumer of the old contract is updated. `raise-pr` and `review-pr`
  are unaffected.
- The parser options match GitHub's extensions.

**Findings**:
- 🟡 major (medium) — Phase 2, Section 3: Known structure-breaking parser
  divergences ship with only a follow-up. Block Phase 3 on HTML-check
  failures, or record them as an accepted known regression.
- 🔵 minor (high) — Phase 2, Section 2: The verification cannot detect lines
  left wrapped. Add a `mode=gfm` `<br>` count, or a gfm-vs-markdown diff.
- 🔵 minor (low) — Phase 1, Section 3: No probes for known divergent
  constructs: alert casing, lines after a table, footnote lazy continuation,
  math.
- 🔵 minor (medium) — Key Discoveries; Phase 3, Section 5: The plan and the
  research disagree on `pulldown-cmark-escape`. Check the lock diff after the
  sync.
- 🔵 suggestion (medium) — Migration Notes: State that future `update-body`
  callers inherit the unwrap, and give the hard-break guidance.

### Standards

**Summary**: The plan follows the repository's documented conventions closely.
The six-point checklist is covered in full, and every cited file, range,
helper and task exists as described. The deviations are small naming and
comment-consistency issues.

**Strengths**:
- The manifest copies the `remote-projection` precedent field for field.
- The workspace dependency carries a rationale and a pin policy, following
  `similar`.
- The pup rule follows the `remote_projection` shape and includes the 443
  alternation.
- The probe pair reuses the existing helpers and a real compliant import.
- The port and test-file names match their siblings.
- The comment policy is applied explicitly.

**Findings**:
- 🔵 minor (high) — Phase 3, Section 2: The adapter misspells a proper noun
  and departs from `GitHub<Role>` naming.
- 🔵 minor (high) — Phase 1, Section 5: The pup rule comment does not match
  its deny list.
- 🔵 minor (medium) — Phase 1, Section 3: `review_fixtures_join` is named for
  where its fixtures came from.
- 🔵 minor (high) — Phase 1, Section 5: The shared-probe comment at
  `test_import_rule.py:916-920` is not updated.
- 🔵 suggestion (medium) — Key Discoveries: The 443 alternation does not list
  every crate (`config`, `config_adapters` and `graphql_test_support` are
  absent).
- 🔵 suggestion (low) — Phase 1, Section 1: Consider a `deny.toml` wrapper ban
  for `pulldown-cmark`.
- 🔵 suggestion (medium) — Phase 1, Section 4: `pub(crate)` on
  `github_extensions` is unnecessary, because a child `tests` module sees
  private items.

### Documentation

**Summary**: The plan updates both places that make claims about
`update-body`'s output, and fixes a stale exit-code claim. Nearly every
citation is accurate. The main gap is the missing CHANGELOG entry for a change
that replaces 0071's contract. The other issues are small citation and
staleness points.

**Strengths**:
- 12 cited line ranges were verified accurate.
- It fixes the SKILL.md exit-1/exit-2 error.
- It correctly leaves the generated reference docs alone.
- The planned comments follow the low-comment norm.
- The describe-pr wording meets the acceptance criterion.

**Findings**:
- 🟡 major (high) — Migration Notes / Phase 3: There is no CHANGELOG entry.
  Add a `### Changed` bullet under `[Unreleased]`.
- 🔵 minor (high) — Current State Analysis: The citation should be
  `SKILL.md:180-181`.
- 🔵 minor (high) — Key Discoveries / Phase 1, Section 5: The README checklist
  is not updated with the `pup.ron:443` obligation.
- 🔵 minor (medium) — Phase 1, Section 5: The shared-probe comment will go
  stale.
- 🔵 minor (medium) — Phase 3, Sections 1-2: The `update_pull_request_body`
  and `github` crate docs are not updated.
- 🔵 minor (medium) — Phase 1, Section 5: The pup rule comment omits the
  filesystem denial.
- 🔵 minor (medium) — Phase 2, Section 4: The plan assumes describe-pr copies
  the corpus record. Add a manual check instead.
- 🔵 suggestion (low) — Phase 3, Section 4: The `collaboration.md` sentence is
  long, and it does not mention that `>` markers and indentation go with the
  break.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-10-13T23:26:00+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Correctness**: SoftBreak followed by End(Link/Image) yields a reversed
  span that duplicates the document — Resolved (orchestrator re-verified
  against 0.13.4; lens traced the fallback through the source)
- 🟡 **Test Coverage**: Config-independence test only checks the two runs
  agree — Resolved
- 🟡 **Compatibility**: Known structure-breaking parser divergences ship with
  only a follow-up — Resolved (structure check now blocks Phase 3)
- 🟡 **Documentation**: No CHANGELOG entry — Resolved
- 🔵 **Correctness**: Trailing-whitespace class differs from the parser's —
  Resolved
- 🔵 **Test Coverage**: unchanged-file body, ordering claim, non-ASCII,
  tabs and multi-break fixtures — Resolved
- 🔵 **Compatibility**: residual-wrap detection, divergence probes,
  `pulldown-cmark-escape` — Resolved
- 🔵 **Architecture**: port pairing, nine parameters, 0320 reuse — Resolved
- 🔵 **Code Quality**: parameter growth, buried `min` invariant, assertion
  messages, pup comment, adapter name — Resolved
- 🔵 **Standards**: adapter name, pup comment, `review_fixtures_join`,
  443 claim, `pub(crate)` — Resolved; shared-probe comment — Partially
  resolved (see below)
- 🔵 **Documentation**: citation, README checklist, doc comments, pup comment,
  corpus record — Resolved; `collaboration.md` wording — Still present and
  now inaccurate (see below)
- 🔵 **Architecture**: infallible port (suggestion) — Still present
- 🔵 **Standards**: `deny.toml` wrapper ban (suggestion) — Still present
- 🔵 **Test Coverage**: golden `cli_surface.rs` for `--help` (suggestion) —
  Still present

### New Issues Introduced

- 🔵 **Documentation**: `collaboration.md` says block structure is kept
  "byte-for-byte" and the CHANGELOG says "posted as written", but `>` markers,
  continuation indentation and trailing whitespace go with each break.
- 🔵 **Test Coverage / Compatibility**: neither Phase 2 GitHub check has a
  positive control; a check that cannot fire reads as a pass. Render `a\nb`
  (excess 1) and `a  \nb` (excess 0) first, count `<br` openings, and feed a
  known-divergent pair through the structure check.
- 🔵 **Compatibility**: the footnote probe `[^1]: a\nb` has no reference, so
  cmark-gfm renders nothing; use `x[^1]\n\n[^1]: a\nb`.
- 🔵 **Compatibility**: the structure normaliser keeps per-render attributes
  (footnote `id` hex suffixes, math renderer ids); render one input twice and
  strip whatever varies.
- 🔵 **Standards**: the generalised shared-probe comment claims every
  whole-crate `denied`-only rule uses the helper; four do not.
- 🔵 **Documentation**: the comment above
  `only_the_consent_policy_names_the_command_runner` still says the regex
  lists every crate but `config`/`config_adapters`; `graphql_test_support` is
  also missing.
- 🔵 **Code Quality**: the link-closing fallback branch would read better on
  a named boolean (`next_event_is_on_continuation_line`); `published` should
  be `prepared_body`.
- 🔵 **Correctness**: the config-independence line is 94 columns, not 95.
- 🔵 **Correctness**: a whitespace-only VT/FF continuation line yields a
  HardBreak at `line_start` that breaks event-stream equality; harmless,
  classify as a parser quirk if hit.
- 🔵 **Test Coverage**: the scratch verifier should copy the normaliser and
  options verbatim rather than re-implement them.
- 🔵 **Documentation**: hard-break guidance lives only in the CHANGELOG, not
  in SKILL.md or `collaboration.md`; the record template has no probe-outcome
  row; the port doc's "so it pairs with" reads as a consequence, not an
  obligation.

Two flagged points reflect explicit user decisions and are not open:
the CHANGELOG entry sits under `### Changed` (documentation and standards
argued for `### Breaking`), and prose uses British "GitHub-flavoured"
(standards argued for the spec's "Flavored").

### Assessment

All four majors and every first-pass minor are resolved or deliberately
deferred, and the link/image fix is verified against the parser source. The
remaining items are minor calibration and wording points, chiefly the
inaccurate "byte-for-byte" claim and the missing Phase 2 positive controls.
The plan is acceptable for implementation; the minors are cheap to fold in
first.
