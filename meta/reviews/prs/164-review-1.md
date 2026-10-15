---
type: "pr-review"
id: "164-review-1"
title: "[0332] Post PR descriptions to GitHub with soft line breaks unwrapped"
date: "2026-10-14T15:56:51+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:164"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "compatibility", "documentation", "standards"]
review_number: 1
pr_number: 164
tags: []
last_updated: "2026-10-14T15:56:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #164 - [0332] Post PR descriptions to GitHub with soft line breaks unwrapped

**Verdict:** COMMENT

A well-built, well-gated change. The unwrap is a pure `&str -> String` core fenced into its own crate, with a domain port, an adapter and a composition-root wiring that respect dependency direction. Fixtures cover LF and CRLF, idempotence and event-stream checks, and a correctness trace found no input that produces wrong output or panics. The findings are all minor or suggestions. They cluster around how the change is announced (CHANGELOG section), how the new port composes with its sibling, and a dialect-neutral public name over GitHub-specific rules.

### Cross-Cutting Themes

- **Behaviour change filed under Changed, not Breaking** (flagged by: standards, documentation, compatibility) — the `CHANGELOG.md` entry retires 0071's byte-for-byte contract and tells users to add hard breaks to trailers or `Closes #n` lines, but sits under `### Changed`. This changelog puts user-action changes under `### Breaking`.
- **Port composition** (flagged by: architecture, code-quality, standards) — the rule that `PullRequestBodyPublishing` must pair with the same forge's `PullRequestBodyUpdate` lives only in a doc comment. `update_pull_request_body` now takes six `&dyn` ports positionally, and the new parameter `publishing` breaks the `body_update` naming parallel. A forge bundle, or a default method on `PullRequestBodyUpdate`, would address all three.
- **GitHub dialect behind a neutral name** (flagged by: architecture, code-quality, compatibility) — `markdown::unwrap_soft_line_breaks` hard-codes `github_extensions()` and `renders_as_soft_on_github`. 0320 (Linear) will need different semantics, so naming the dialect now keeps that change additive to the pinned API.
- **Dialect-divergence evidence lives outside CI** (flagged by: test-coverage, correctness, compatibility) — the corpus verification ran from an uncommitted scratch crate. The event-stream oracle reuses the production predicate. Alert rendering on the PR page and `$$` display math remain unverified.

### Tradeoff Analysis

- **Compatibility vs simplicity**: compatibility suggests a `--verbatim` opt-out on `update-body` to keep the old contract available. One forge and one caller exist today, so a Breaking entry plus the documented hard-break guidance is enough. Add the flag only when a caller asks for it.
- **Structural safety vs YAGNI**: the PR deliberately keeps `prepare_for_publishing` infallible and separate while one forge exists. A cheap middle ground is a default identity `prepare_for_publishing` on `PullRequestBodyUpdate` that `OctocrabClient` overrides. That makes the pairing structural without a new bundle type.

### Strengths

- ✅ `pulldown-cmark` is fenced into `markdown` the same way `serde-saphyr` is fenced into `document`: an exact pin with default features off, plus a `cargo-deny` `wrappers` ban.
- ✅ Dependency direction is correct. `collaboration` owns the port, `github` implements it using the library, and only `accelerator-collaboration` names the concrete type.
- ✅ The new crate meets all seven library-crate checklist items: role declaration, `denied`-only pup rule with a probe pair, public-API pin, deny ban, notices entry and command-runner alternation.
- ✅ The PR closes a gate gap: `graphql_test_support` was missing from the command-runner alternation, and the README checklist now names that step.
- ✅ The span logic was traced clean. It scans the same whitespace set pulldown-cmark uses, slices only at ASCII boundaries so it is UTF-8 safe, and its spans never overlap. The `min(prefix, next event)` cap stops deep-indented `>` being swallowed.
- ✅ The HardBreak reclassification exactly mirrors cmark-gfm's two-spaces-or-backslash rule, which is a strict subset of pulldown-cmark's, so no break GitHub renders as hard is ever joined.
- ✅ Every fixture checks exact output, idempotence and event-stream equality under both LF and CRLF. Characterisation tests pin pulldown-cmark's undocumented `SoftBreak` and `HardBreak` offset ranges.
- ✅ Both subtle branches in `continuation_start` survive mutation: each is pinned by a named fixture.
- ✅ The `MarkedPublishing` fake makes the use-case test catch passing the raw body instead of the prepared one. The e2e tests assert the decoded PATCH payload and that the body file is unchanged.
- ✅ The `--help` golden pins the CLI contract the skills depend on. The describe-pr exit-2 correction matches `run_update_body`.
- ✅ New comments explain why: the exact-pin rationale, the deny ban, and the pup reference to the existing grouped-import note.
- ✅ The `rustls` bump is a semver patch inside the exact pin. `cargo update --precise` moved nothing else, and the notices were regenerated.

### General Findings

- 🔵 **Test Coverage**: Corpus-level idempotence and structure checks are not in CI. The verifier that ran 115 real descriptions was not committed, so a pulldown-cmark bump re-runs only the hand-picked fixtures. Commit the offline half (idempotence plus event-stream equality over a checked-in corpus) as a `markdown` test.
- 🔵 **Correctness / Compatibility**: The unwrap is only as correct as pulldown-cmark and cmark-gfm agreeing on block structure. Alert variants (trailing whitespace, CRLF, nested in a list) are not probed, and alert rendering on the PR page is unconfirmed because the REST `/markdown` endpoint does not render alerts. Add those fixtures and check them by eye on a live PR, alongside the unticked full-width check.
- 🔵 **Test Coverage**: Bare-CR line endings are not exercised. The correctness trace says they are handled, but no fixture pins it. Add a `\r` variant to `assert_unwraps`, or record bare CR as out of scope.

### Additional Findings

- 🔵 `cli/markdown/src/lib.rs:48` — Dense `filter_map` with a guarded slice pattern; extract `soft_line_break_span` (code-quality)
- 🔵 `cli/collaboration-cli/tests/end_to_end.rs:680` — The line-width-conventions test passes trivially; the binary reads none of those files (test-coverage)
- 🔵 `skills/github/describe-pr/SKILL.md:177` — The hard-break constraint is documented only at the posting step, not where the description is generated (compatibility)
- 🔵 `cli/markdown/src/lib.rs:85` — Parameter `line_ending` receives the line break's start; rename it `line_break_start` (code-quality)
- 🔵 `cli/markdown/src/lib.rs:8` — `mod tests;` is declared before the public item; the codebase's precedent puts it at the end (standards)

## Inline Comments

### `cli/markdown/src/lib.rs:11-22` — GitHub-dialect rules behind a dialect-neutral public name
**Severity**: minor | **Confidence**: high | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

`markdown::unwrap_soft_line_breaks` reads as a dialect-neutral operation, but it hard-codes `github_extensions()` and decides breaks with `renders_as_soft_on_github`, which applies cmark-gfm's two-spaces-or-backslash rule. GitHub-specific knowledge therefore sits in the technical library under a generic name.

**Impact**: The PR already expects 0320 (Linear) to need different semantics. A later caller picking this function by its name would inherit GitHub's whitespace rule silently, and adding the dialect then means renaming a pinned public API.

**Suggestion**: Name the dialect now, for example `unwrap_soft_line_breaks_for_github` or `unwrap_soft_line_breaks(markdown, Dialect::GitHub)`. 0320 then adds a sibling instead of renaming.

---

### `CHANGELOG.md:434-442` — Behaviour change requiring user action filed under Changed, not Breaking
**Severity**: minor | **Confidence**: medium | **Lens**: standards, documentation, compatibility

🔵 **Standards / Documentation / Compatibility**

This entry retires 0071's byte-for-byte posting contract and ends with a required action: a trailer or `Closes #n` line must gain a hard break or become a list item to keep rendering on its own. The Unreleased `### Breaking` section is where this changelog collects changes that need users to adapt, such as the credential-runner and refusal-code entries.

**Impact**: Users who scan only Breaking before upgrading will miss that custom templates or instructions with line-per-newline layouts now render collapsed on GitHub, with no error.

**Suggestion**: Move the entry to `### Breaking`, or add a Breaking bullet that points to it. Keep the hard-break and list-item guidance either way.

---

### `cli/collaboration/src/lib.rs:119-123` — Forge pairing of the publishing and update ports rests on a doc comment
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

The doc comment says to compose `PullRequestBodyPublishing` only with the same forge's `PullRequestBodyUpdate`, but `update_pull_request_body` takes them as independent `&dyn` arguments, and the composition root wires them separately (`&client` and `&GitHubPullRequestBodyPublishing`).

**Impact**: Once a second forge exists, a mismatched pairing compiles and posts a silently mis-rendered body. No gate catches it.

**Suggestion**: Make the pairing structural. The cheapest option is a default identity `prepare_for_publishing` on `PullRequestBodyUpdate` that the GitHub client overrides. Alternatively, a forge bundle could supply both operations from one adapter value.

---

### `cli/markdown/src/tests.rs:34-41` — Event-stream oracle reuses the production soft-break predicate
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

`normalised_events` decides which events become a space by calling the production `renders_as_soft_on_github`. If that predicate misclassified a HardBreak, the input and output streams would be normalised the same wrong way, and the event-stream check would still pass.

**Impact**: The event-stream assertion looks like an independent oracle but cannot detect a regression in the subtlest rule in the crate. Only the hand-written expected strings guard it.

**Suggestion**: Give the normaliser its own written-out GitHub-softness rule, or add a table-driven test of `renders_as_soft_on_github` over the HardBreak marker shapes.

---

### `cli/pup.ron:462` — Command-runner rule still fails open on a hand-maintained crate alternation
**Severity**: minor | **Confidence**: medium | **Lens**: architecture

🔵 **Architecture**

This PR fixes two omissions (`markdown`, `graphql_test_support`) and adds a README checklist step. A crate missing from this regex alternation still goes unchecked, and nothing reports it.

**Impact**: The `graphql_test_support` gap went unnoticed until now, so a checklist step alone does not reliably guard this boundary.

**Suggestion**: Add a unit test, alongside `tests/unit/tasks/test_crate_dependencies.py`, that asserts every `[workspace].members` crate (in snake_case) appears in this alternation or on an explicit exemption list.

---

### `cli/collaboration/src/update_body.rs:41` — Parameter `publishing` breaks the `body_update` naming parallel
**Severity**: minor | **Confidence**: medium | **Lens**: standards

🔵 **Standards**

Each other port parameter is the trait name in snake_case: `repository_lookup`, `pull_request_existence`, `body_update`. The new `publishing: &dyn PullRequestBodyPublishing` drops the `body_` part that its sibling keeps.

**Impact**: The signature is pinned in the public-API snapshot, so the inconsistency becomes part of the contract.

**Suggestion**: Rename it `body_publishing` and re-bless the pin.

---

### `cli/github/src/pull_request_body_publishing.rs:1` — New module lacks the `//!` header its siblings carry
**Severity**: minor | **Confidence**: medium | **Lens**: standards

🔵 **Standards**

`octocrab_client.rs` and `remote_url_recognizer.rs` each open with a `//!` line naming the `collaboration` port they implement. This module starts directly with `use`.

**Suggestion**: Add `` //! The GitHub implementation of `collaboration`'s `PullRequestBodyPublishing` port. ``

---

### `cli/markdown/src/lib.rs:34-40` — GitHub `$$` display math is not recognised, so multi-line math is joined
**Severity**: minor | **Confidence**: low | **Lens**: compatibility

🔵 **Compatibility**

GitHub renders `$$ … $$` blocks as math, but `github_extensions()` leaves `Options::ENABLE_MATH` off. A multi-line display-math block therefore parses as a paragraph and its newlines are joined. LaTeX mostly ignores that whitespace, but a `%` comment mid-block would comment out everything joined after it. The REST `/markdown` corpus check may not exercise GitHub's math rendering.

**Impact**: A description with multi-line display math could post a truncated equation.

**Suggestion**: Enable `Options::ENABLE_MATH` so math becomes single events with no `SoftBreak` inside. Add a fixture such as `"$$\na % c\nb\n$$\n"` asserting the block is unchanged, and update `dollar_delimited_math_joins_as_text`.

---

### `cli/collaboration/src/update_body.rs:41` — Use-case signature widens by one positional port per capability
**Severity**: suggestion | **Confidence**: medium | **Lens**: architecture, code-quality

🔵 **Architecture / Code Quality**

`update_pull_request_body` now takes six `&dyn` ports plus `root`, `pull_number` and `body`. The composition root passes `&client` three times, then its sibling adapter. The tests needed a second helper (`update_published_by`), with a different argument order, to absorb the new parameter.

**Impact**: Each forge capability changes the pinned API and every call site, and same-shaped adjacent ports are easy to mis-order.

**Suggestion**: Group the forge-side ports into a `PullRequestForge` bundle or trait supplied by the adapter. This also resolves the pairing concern on `PullRequestBodyPublishing`.

---

### `cli/markdown/src/lib.rs:11-12` — Rustdoc summary promises more byte preservation than the function gives
**Severity**: suggestion | **Confidence**: medium | **Lens**: documentation, correctness

🔵 **Documentation / Correctness**

The summary line says the function leaves "every other byte as written", yet the span also absorbs trailing whitespace, `>` markers and indentation. A continuation line holding only VT/FF (`foo\n \x0B\nbar`) is dropped entirely, as `a_whitespace_only_continuation_line_joins_into_both_neighbours` pins. Rustdoc listings and hovers show only the summary line.

**Impact**: The output is harmless, but a future caller, such as 0320's dialect-aware entry point, could rely on a stronger guarantee than the code provides.

**Suggestion**: Make the summary match the span rule. For example: "Replaces each GFM soft line break, with the whitespace and container markers around it, by a single space."

---

## Per-Lens Results

### Architecture

**Summary**: The change is structurally sound and follows hexagonal layering. A new technical library wraps `pulldown-cmark` behind one function and is fully gated. The domain port is implemented in `github` and wired in at the composition root. Concerns are minor evolutionary-fitness issues: the pairing invariant rests on a doc comment, GitHub rules sit behind a dialect-neutral name, and the pup alternation is hand-maintained.

**Strengths**: parser fenced like `serde-saphyr`/`document`; correct dependency direction; pure functional core held by a `denied`-only pup rule; command-runner gap closed; CLI golden; explicit acknowledgement of the `String` vs `Result` tradeoff and of 0320.

**Comments**:
- `cli/collaboration/src/lib.rs:119-123` (minor, medium) — The forge pairing rests on a doc comment.
- `cli/markdown/src/lib.rs:11-22` (minor, high) — GitHub-dialect rules sit behind a dialect-neutral name.
- `cli/pup.ron:462` (minor, medium) — The command-runner rule depends on a hand-maintained alternation; add a fail-closed test.
- `cli/collaboration/src/update_body.rs:41` (suggestion, medium) — The use-case signature widens per port; consider a `PullRequestForge` bundle.

**General Findings**: none.

### Correctness

**Summary**: The span logic in `cli/markdown/src/lib.rs`, traced against pulldown-cmark's break emission and cmark-gfm's hard-break rule, produces no wrong output and no panics. Spans are ordered and non-overlapping, and slices fall on UTF-8 boundaries. CRLF, bare CR, blockquotes, lists, nested containers, link closers and indented `>` are all handled. The port wiring and the exit-2 doc correction are correct.

**Strengths**: the HardBreak reclassification mirrors cmark-gfm's `handle_newline`; scans are ASCII-only, so UTF-8 safe; no overlap is possible; the `min` cap and the `End` fallback reach only whitespace and `>`; replacing a newline with a space preserves inline semantics; the exit-2 claim matches `run_update_body`.

**Comments**:
- `cli/markdown/src/lib.rs` (suggestion, low; anchored by the agent outside the diff, so merged into the rustdoc comment at `cli/markdown/src/lib.rs:11-12`) — A continuation line holding only VT/FF is deleted rather than preserved, contradicting "every other byte as written".

**General Findings**:
- (minor) Correctness rests on pulldown-cmark and cmark-gfm agreeing on block structure. Probe alert variants and check them on a live PR page.

### Code Quality

**Summary**: The change is small, well-factored and readable. The concerns are about design rather than defects: the port pairing is enforced only by a doc comment, the use case's parameter list keeps growing, a generic name encodes GitHub rules, and the span closure is dense.

**Strengths**: pure function fenced by pup and deny; the `next_event_is_on_continuation_line` local stands in for a comment; small, domain-named helpers; e2e mock setup de-duplicated (~50 lines); the stateless adapter's doc says why; minimal, behaviour-named test doubles.

**Comments**:
- `cli/collaboration/src/lib.rs:119-122` (minor, medium) — The port pairing is enforced only by a doc comment.
- `cli/collaboration/src/update_body.rs:41` (suggestion, medium) — The parameter list has grown to nine; bundle the forge ports.
- `cli/markdown/src/lib.rs:22` (minor, medium) — The generic name encodes GitHub-specific rules.
- `cli/markdown/src/lib.rs:48-66` (suggestion, medium) — The `filter_map` is dense; extract `soft_line_break_span`.
- `cli/markdown/src/lib.rs:85` (suggestion, low) — `line_ending` receives the line break's start; rename it `line_break_start`.

**General Findings**: none.

### Test Coverage

**Summary**: Coverage is strong and proportional to the risk. About 30 fixtures each run under LF and CRLF with exact, idempotence and event-stream checks, and characterisation tests pin offset behaviour. The port is tested at every layer, with a pup probe pair. The gaps: the corpus check is not reproducible in CI, the oracle reuses the production predicate, and one e2e test passes trivially.

**Strengths**: multi-property `assert_unwraps`; mutation-resistant `continuation_start` branches; boundary fixtures on both sides of the HardBreak rule; offset characterisation tests; an interleaved preserved-constructs fixture; a `MarkedPublishing` fake; e2e assertions on the PATCH payload; de-duplicated mock setup; a pup probe pair with a real compliant control.

**Comments**:
- `cli/markdown/src/tests.rs:34-41` (minor, medium) — The event-stream oracle reuses the production predicate. The agent anchored this at 968-984, which are diff-patch line numbers; it was re-anchored to the file lines.
- `cli/collaboration-cli/tests/end_to_end.rs:680` (suggestion, medium) — The line-width-conventions test passes trivially. The agent anchored this at 411-437; it was re-anchored to the test's line.

**General Findings**:
- (minor) Corpus-level idempotence and structure checks are not in CI.
- (suggestion) Bare-CR line endings are not exercised.

### Compatibility

**Summary**: The PR changes what `update-body` sends to GitHub, but the CLI flags, exit codes and body file stay the same. The Rust API change is deliberate, re-pinned, and has a single caller. Two issues remain: the changelog section, and `$$` display math, which is joined because math parsing is off. The `rustls` bump and the `pulldown-cmark` pin are safe.

**Strengths**: CLI golden; a clean `rustls` patch bump; exact-pinned, wrapper-fenced parser; deliberate public-API re-bless; dialect gap addressed with LF/CRLF fixtures and the `/markdown` corpus; exit-2 correction.

**Comments**:
- `CHANGELOG.md:434-442` (minor, medium) — The entry belongs under Breaking. Also consider a `--verbatim` opt-out.
- `cli/markdown/src/lib.rs:34-40` (minor, low) — `$$` display math is joined; enable `ENABLE_MATH`.
- `skills/github/describe-pr/SKILL.md:177-180` (suggestion, medium) — The hard-break constraint is documented only at the posting step.

**General Findings**:
- (suggestion) GitHub alert rendering on the PR page is unconfirmed.

### Documentation

**Summary**: The user-facing docs keep up with the change across the CHANGELOG, the describe-pr SKILL, docs-site and the `tasks/README.md` checklist. The exit-2 correction is accurate, and the new comments follow the comment policy. The two issues are small accuracy and placement points.

**Strengths**: the stale "you see exactly what gets posted" claim is removed; the exit-2 correction was verified; migration guidance is consistent across three surfaces; the README count is updated to seven; comments explain why; the rustdoc captures the parser disagreement; the port doc states the usage constraint.

**Comments**:
- `CHANGELOG.md:434-442` (minor, medium) — The entry belongs under Breaking.
- `cli/markdown/src/lib.rs:11-12` (suggestion, medium) — The rustdoc summary overstates byte preservation.

**General Findings**: none.

### Standards

**Summary**: The `markdown` crate meets all seven library-crate checklist items and copies the existing pup and deny patterns. Port and adapter names follow precedent. The remaining findings are small consistency points.

**Strengths**: checklist complete; pup rule matches `remote_projection_extracts_json_only`; deny ban matches `serde-saphyr`; the `graphql_test_support` gap is closed; names match `PullRequestBodyUpdate` and `GitHubRemoteUrlRecognizer`; notices, CHANGELOG and docs follow the existing patterns.

**Comments**:
- `cli/collaboration/src/update_body.rs:41` (minor, medium) — Rename `publishing` to `body_publishing`.
- `cli/github/src/pull_request_body_publishing.rs:1` (minor, medium) — The `//!` module header is missing.
- `cli/markdown/src/lib.rs:8-9` (suggestion, low) — Move `mod tests;` to the end of `lib.rs`.
- `CHANGELOG.md:434-442` (minor, medium) — The entry belongs under Breaking.

**General Findings**: none.

---
*Review generated by /accelerator:review-pr*
