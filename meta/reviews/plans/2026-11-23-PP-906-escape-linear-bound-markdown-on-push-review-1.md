---
type: "plan-review"
id: "2026-11-23-PP-906-escape-linear-bound-markdown-on-push-review-1"
title: "Plan Review: Escape Linear-Bound Markdown on Push Implementation Plan"
date: "2026-11-23T21:55:58+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-11-23-PP-906-escape-linear-bound-markdown-on-push"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["correctness", "test-coverage", "architecture", "code-quality", "compatibility", "safety", "usability"]
review_number: 1
review_pass: 8
tags: ["sync", "linear", "markdown"]
last_updated: "2026-11-24T00:19:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Escape Linear-Bound Markdown on Push Implementation Plan

**Verdict:** REVISE

The design is sound. A pure escaper in the `markdown` technical library splices only backslashes at source offsets, and the adapter's `RemoteTracker` impl applies it. Digests and `BodyDialect` stay untouched, and two live-capture gates de-risk the main assumption about Linear's behaviour before any code ships. It needs revising for three reasons. Several concrete specifications would not go green as written: the simulator change, `<span>` and the backslash-stripping property. One shipped push path is left unescaped. The plan also relies on a drift-detection signal that the sync design cannot produce.

### Cross-Cutting Themes

- **The simulator cannot reproduce `X11` or `X14`** (flagged by: correctness, test-coverage, code-quality, architecture). `autolink_dotted_tokens` uses `rsplit_once('.')` with a dot-free name, so `jql.rs:167` and `a.b.io` never link. Phase 2's automated criterion cannot pass, and the escaper and simulator end up with two separate dotted-token grammars.
- **Duplicated parser options with a sync test that cannot fail** (flagged by: architecture, code-quality). `document::markdown::events` is already public and free of `pulldown-cmark` types, with the same three flags. The proposed agreement test parses only with the escaper's own options.
- **Push baselines Linear's read-back, so drift is silent** (flagged by: safety, usability, architecture). The Migration Notes say a newly linked token "surfaces as a conflict". The work item itself says push records the read-back, which is exactly how PP-180 and the other affected items went unnoticed.
- **`UPDATE_NO_FILE` has no number and the wrong registration site** (flagged by: compatibility, usability). `captured-exit-codes.txt` is the frozen legacy capture. New codes follow the `SEARCH_CAP_HIT` pattern instead. 115 is the next free slot.
- **The `assert_escapes` "remove inserted backslashes" property is ill-defined** (flagged by: correctness, test-coverage). It cannot tell inserted escapes from existing ones (`a.b\.io`). A subsequence walk fixes it.
- **Gaps in the docs and skill surface** (flagged by: usability, compatibility). No exit-code table exists. The real targets are `update-linear-issue/SKILL.md` and `cli_surface.golden`.

### Tradeoff Analysis

- **Safety vs simplicity (post-push equivalence check)**: Safety and usability want a read-back equivalence check after each Linear push, so drift is not silent. That adds behaviour to the sync engine, which this item deliberately leaves alone. Recommendation: at minimum, correct the Migration Notes. A warn-only check can be its own follow-up work item.
- **Architecture vs dependency simplicity (`markdown` → `document`)**: Building on `document::markdown::events` removes the duplicated options, but adds a dependency between two libraries and ties the escaper to `MarkdownEvent`'s shape. Recommendation: adopt it, after checking that `MarkdownEvent` keeps backslash-escape `Text` ranges unmerged.

### Findings

#### Major

- 🟡 **Architecture**: `linear create <work-item-file>` pushes work-item bodies unescaped
  **Location**: Overview / Send path / What We're NOT Doing
  The positional work-item-file mode (`main.rs:412-426`) sends a local work item's body through `create_op`, and the shipped `/create-linear-issue` skill uses exactly that path. Only `--title`/`--body-file` needs to stay verbatim.
- 🟡 **Correctness + Test Coverage + Code Quality**: The simulator change cannot reproduce `X11`/`X14`
  **Location**: Phase 2, Section 5
  The simulator needs multi-label names and a `:digits` suffix, or the two fragments must stay out of `recorded_rewrites()`. `X12` and `X13` pass before the change, so "each fails" is also inaccurate.
- 🟡 **Architecture + Code Quality**: The option set is duplicated although `document::markdown::events` exposes it
  **Location**: Phase 4, Sections 1 and 3
  The agreement test cannot see `document`'s options, so drift goes undetected.
- 🟡 **Correctness**: Inline HTML content is `Text`, so `<span>crates.io</span>` would be escaped
  **Location**: Phase 5, Tests first
  The expectation contradicts the text-run algorithm. Decide whether the invariant is "HTML tags are never escaped" (escaping inside is harmless) or track open inline elements.
- 🟡 **Correctness**: The digit rule may insert a backslash inside a multi-line inline HTML tag
  **Location**: Phase 4, Section 3
  `<span title="owned by\n0160. x">` is a case nothing pins down. Characterise it and skip breaks inside `InlineHtml` spans.
- 🟡 **Correctness**: The capture test assumes every link is `[t](<http://t>)`, and the gate checks only last labels
  **Location**: Phase 5 capture test; Phase 2 gate
  `a@b.io` probably reads back as a mailto link over the whole address. The gate should compare the exact span and link form for `X16`-`X19` and define a fallback.
- 🟡 **Test Coverage**: `assert_escapes` omits the CommonMark-equivalence check
  **Location**: Phase 4, Section 2
  `assert_unwraps` compares normalised event streams, and `assert_escapes` should too.
- 🟡 **Test Coverage**: No case puts multibyte text before a dotted token
  **Location**: Phase 5, Tests first
  Character-versus-byte offset bugs, or a slice on a non-character boundary, would panic on real bodies containing `—` or accented text.
- 🟡 **Safety + Usability**: Push baselines the read-back, so escaper defects and linkifier drift stay silent
  **Location**: Migration Notes; Phase 6
  The "surfaces as a conflict" claim is false. Correct it, or add a post-push equivalence warning.
- 🟡 **Safety**: The repair push can overwrite unpulled remote edits
  **Location**: Phase 7, Step 4
  `push_update` does no freshness check. Require a targeted sync reporting `synced` first, and confirm the remote differs only by the rewrites.
- 🟡 **Safety**: No copy of the remote description is kept before the repair
  **Location**: Phase 7, Steps 2 and 4
  Save each item's allowlisted `linear show` output until the web-UI check passes.
- 🟡 **Compatibility + Usability**: `UPDATE_NO_FILE` has no number and would land in the captured-legacy fixture
  **Location**: Phase 1, Section 4
  Pin 115, add an `_is_115_and_uncontested` test, update `stderr_diagnostics.rs`, and do not touch `captured-exit-codes.txt`.

#### Minor

- 🔵 **Correctness**: The rule takes only each word's longest prefix, so it misses `Note:crates.io` and `foo..bar.io`
  **Location**: Phase 5, Implementation step 2
- 🔵 **Correctness + Test Coverage**: The remove-backslashes property is ambiguous when the input already holds escapes
  **Location**: Phase 4, Section 2
  Restate it as: the input is a subsequence of the output, and every extra byte is `\`.
- 🔵 **Correctness**: GFM's trailing-punctuation shedding must repeat
  **Location**: Phase 5, Implementation step 1
- 🔵 **Test Coverage**: The oracle comparing escaped and linked tokens is underspecified
  **Location**: Phase 5 capture test
- 🔵 **Test Coverage**: The mock read-back comes from the escaper, and the `PromotionRetitle` assertions are unclear
  **Location**: Phase 6, Section 3
- 🔵 **Test Coverage**: The Jira assertion is a loose substring check
  **Location**: Phase 6, Section 3
- 🔵 **Test Coverage**: Edge cases are missing: two-space hard break, tab, lazy blockquote, image alt, `\\.`, `&#46;`
  **Location**: Phases 4 and 5 tables
- 🔵 **Code Quality**: `escaped_at` repeats `github`'s splice loop
  **Location**: Phase 4, Section 3
- 🔵 **Code Quality**: `dotted_tokens` is specified only in prose
  **Location**: Phase 5, Implementation
  Name the signatures for `TextRun::from_events`, `SchemeUrl::spans` and `DottedToken::in_word`.
- 🔵 **Code Quality**: Body-file reading is duplicated between create and update
  **Location**: Phase 1, Section 3
- 🔵 **Architecture**: The `markdown` tests reach into the `work-adapters` fixtures
  **Location**: Phase 5 capture tests
- 🔵 **Architecture**: The seam shared with the simulator is only the TLD lookup
  **Location**: Phase 2, Sections 4 and 5
- 🔵 **Compatibility**: Escapes stored remotely reach local files on pull
  **Location**: Desired End State / Phase 3
- 🔵 **Compatibility**: Teammates on older plugin versions can re-corrupt repaired items
  **Location**: Phase 7
- 🔵 **Safety**: No runtime fallback if the escaper changes a body's meaning
  **Location**: Phase 6, Implementation
- 🔵 **Safety**: The capture target is inconsistent (PP workspace or scratch team)
  **Location**: Implementation Approach; Phases 2 and 3
- 🔵 **Usability**: Raw `--body-file` silently skips the escaping, and the help text and skill do not say so
  **Location**: Phase 1
- 🔵 **Usability**: The unreadable-file error omits the path
  **Location**: Phase 1, Section 3
- 🔵 **Usability**: Nothing in the plan uses `linear update --body-file`
  **Location**: Phase 1, Overview

#### Suggestions

- 🔵 **Correctness**: No fallback if the `Text` range around `\.` includes the backslash
  **Location**: Phase 4, Section 1
- 🔵 **Correctness**: The snapshot shape test must accept the digits and `-` in `XN--` labels
  **Location**: Phase 2, Section 4
- 🔵 **Test Coverage**: Add a whole-document capture comparison
  **Location**: Phase 5 capture tests
- 🔵 **Code Quality**: Escape offsets are bare `usize`, where an `Escape` newtype would carry their meaning
  **Location**: Phase 4, Section 3
- 🔵 **Usability**: The Phase 7 detection steps give no concrete commands
  **Location**: Phase 7, Steps
- 🔵 **Usability + Architecture**: No refresh procedure or owner for the TLD snapshot
  **Location**: Migration Notes

### Strengths

- ✅ Escaping happens only in `impl RemoteTracker for LinearClient`, so every sync, promotion and `work --push` path inherits it, while `create_op`/`update_op` stay verbatim.
- ✅ The escaper is a pure, idempotent `&str -> String` function that changes only parsed text. A pulled `0160\.` never becomes `0160\\.`.
- ✅ Every insertion sits in front of an ASCII `.` or `)` between non-delimiter characters, so splices fall on UTF-8 boundaries and cannot change emphasis or list structure.
- ✅ The digest is neutral by construction: `as_pushed` stays identity and `DigestRecipe::CURRENT` is not bumped, and `golden_digests` guards both.
- ✅ Characterisation tests pin `pulldown-cmark` 0.13.6's behaviour before the rules depend on it.
- ✅ The tests have two independent oracles: the hand-escaped capture and Linear's own read-back.
- ✅ The push matrix uses real clients over `MockHTTPServer` across every tracker and entry point, and asserts the exact bytes sent.
- ✅ The live-capture gates stop the work before any production change if Linear contradicts the assumptions, and each phase can merge on its own.
- ✅ The new dependency edges pass the crate-role and cargo-deny wrapper rules, and no second parser crate is added.

### Recommended Changes

1. **Escape the work-item-file mode of `linear create`** (addresses: unescaped `linear create <work-item-file>`). Add the escape there, add a flow test for each mode, and record the boundary in What We're NOT Doing.
2. **Rework Phase 2's simulator step** (addresses: the simulator cannot reproduce `X11`/`X14`; the shared seam). Either list the multi-label and `:digits` changes, or keep `X11` and `X14` out of `recorded_rewrites()`. Fix the "each fails" claim.
3. **Build the rules on `document::markdown::events`** (addresses: duplicated options). Drop `extensions()` and rewrite the characterisation tests against `MarkdownEvent`.
4. **Settle the inline-HTML behaviour** (addresses: the `<span>` contradiction; the digit rule inside HTML). Pick an invariant, add characterisation and unchanged cases for multi-line tags.
5. **Strengthen `assert_escapes`** (addresses: missing equivalence check; ambiguous property). Add a normalised-event equivalence check, and restate the property as a subsequence walk.
6. **Add test cases** (addresses: multibyte text; edge cases; repeated GFM shedding; longest-prefix tokens). Cover `naïve — crates.io`, `Note:crates.io`, `foo..bar.io`, `(https://crates.io).`, the hard-break and tab variants, `\\.` and `&#46;`.
7. **Tighten the gate and capture oracle** (addresses: link-form assumption; underspecified oracle). Compare exact spans and http/mailto forms for `X16`-`X19`, define a fallback, and define how both sides of the comparison are extracted.
8. **Correct the Migration Notes on drift** (addresses: silent baselining; no refresh procedure). State that drift is silent, name how it would be noticed, and add a snapshot refresh procedure to the README.
9. **Harden Phase 7** (addresses: overwriting unpulled edits; no remote copy; older plugin versions). Run a pre-push targeted sync, save the remote output, and repair only after the release has reached every pusher.
10. **Pin `UPDATE_NO_FILE = 115` and name the docs targets** (addresses: exit code; docs surface; raw-flag warning; error omits path). Use the `SEARCH_CAP_HIT` pattern, update `update-linear-issue/SKILL.md` and `cli_surface.golden`, include the path in the error, and warn in the help text that the flag skips escaping.

## Per-Lens Results

### Correctness

**Summary**: The core logic holds up: offsets are spliced in the source, delimiters are ASCII-only, existing escapes are detected in parsed text, and the digest stays neutral. The main issues: the `<span>` expectation contradicts the algorithm, the simulator cannot reproduce `X11`/`X14`, the digit rule may reach into multi-line inline HTML, and the capture test assumes a single link form.

**Strengths**: parsed-text escaping prevents `\\.`; ASCII insertions are UTF-8 safe; `continuation_start` avoids the trap of events whose range starts on the previous line; insertions cannot alter flanking or structure; link depth is tracked on events; promotion's retitle goes through `update`.

**Findings**:
- major/high, Phase 5: The content of inline HTML is `Text`, so `<span>crates.io</span>` gets escaped.
- major/high, Phase 2 §5: The simulator cannot reproduce `X11`/`X14`.
- major/medium, Phase 4 §3: The digit rule may escape inside a multi-line inline HTML tag.
- major/medium, Phase 5 / Phase 2 gate: The capture test assumes `[t](<http://t>)`, and the gate checks only last labels.
- minor/medium, Phase 5 §2: Taking only the longest prefix misses tokens after `:` or `..`.
- minor/high, Phase 4 §2: The remove-backslashes property is ambiguous when the input has escapes.
- minor/medium, Phase 5 §2: GFM's trailing-punctuation shedding must repeat.
- suggestion/medium, Phase 4 §1: No fallback if the `Text` range includes the backslash.
- suggestion/medium, Phase 2 §4: The shape test must accept the characters in `XN--` labels.

### Test Coverage

**Summary**: A strong strategy, with characterisation tests first, two oracles and a real-client matrix. The gaps: `assert_escapes` does not check equivalence, the simulator cannot reproduce `X11`/`X14`, the token oracle is underspecified, and no multibyte offset case exists.

**Strengths**: characterisation tests; two independent oracles; `assert_unwraps` reuse (LF/CRLF, idempotence); a digit-rule table covering every container; the boundary tested both ways; a direct regression test for corrupted remotes; the `golden_digests` guard; mocking at the HTTP layer.

**Findings**:
- major/high, Phase 4 §2: `assert_escapes` omits the CommonMark-equivalence check.
- major/medium, Phase 5 §1: No multibyte text before a dotted token.
- major/medium, Phase 2 §5: The simulator cannot reproduce `X11`/`X14`.
- minor/medium, Phase 5 §1: The escaped-versus-linked oracle is underspecified (`X11`, `X19`).
- minor/medium, Phase 6 §3: The mock read-back comes from the escaper, and the `PromotionRetitle` assertions are unclear.
- minor/medium, Phase 6 §3: The Jira assertion is a loose substring check.
- minor/medium, Phases 4–5: Missing edge cases (two-space break, tab, lazy blockquote, image alt, `\\.`, `&#46;`).
- suggestion/medium, Phase 5 §1: Add a whole-document capture comparison.

### Architecture

**Summary**: The escape sits where the codebase already puts dialect transforms, built on a pure library function. The boundary misses `linear create <work-item-file>`, and the escaper duplicates a parser configuration that `document::markdown::events` already exposes.

**Strengths**: the escape fits the `RemoteTracker` contract and the `github`/Jira precedents; a functional core; rule composition that is open to extension; valid dependency edges; `as_pushed` identity is justified; PP-876 coordination; phase gating.

**Findings**:
- major/high, Overview / NOT Doing: `linear create <work-item-file>` pushes unescaped.
- major/high, Phase 4: Duplicated options although `document::markdown::events` exists.
- minor/medium, Phase 2: The seam shared with the simulator is only the TLD lookup.
- minor/medium, Phase 5: The `markdown` tests reach into the `work-adapters` fixtures.
- suggestion/medium, Migration Notes: No owner or trigger for keeping the snapshot and captures aligned.

### Code Quality

**Summary**: A well-structured plan with a single choke point and parsed-event rules. The risks are duplicated knowledge (options, simulator grammar, splice loop) and a dotted-token unit specified only in prose.

**Strengths**: event-based rules; a single application point; one module per rule; domain-named types; `requested_description` matches the existing idiom; a small public API checked by `public-api`.

**Findings**:
- major/medium, Phase 4 §3: Duplicated options with a sync test that cannot fail.
- major/high, Phase 2 §5: The simulator change is understated and implies a second grammar.
- minor/high, Phase 4 §3: `escaped_at` repeats the `github` splice loop.
- minor/medium, Phase 5 §2: The most complex unit is specified in prose only.
- minor/medium, Phase 1 §3: Body-file read-and-report is duplicated between create and update.
- suggestion/low, Phase 4 §3: Escape positions are bare `usize`.

### Compatibility

**Summary**: Sound. The raw commands and other trackers are unchanged, the digests are preserved, and the public API only grows. The gap is the unnumbered `UPDATE_NO_FILE` and its proposed registration.

**Strengths**: the escape is confined to `RemoteTracker`; digests unchanged; the public API only grows; dependency edges valid; the parser pin is backed by characterisation tests; Linear's behaviour is checked with live captures; `LazyLock` works within MSRV 1.90.

**Findings**:
- major/high, Phase 1 §4: `UPDATE_NO_FILE` has no number, and its registration would break the captured contract.
- minor/medium, Desired End State / Phase 3: The pull-side effect of stored escapes is not stated.
- minor/medium, Phase 7: Older plugin versions can re-corrupt repaired items.
- suggestion/low, Phase 1 §2: The skill hint does not mention `--body-file`.

### Safety

**Summary**: The change is additive with a small blast radius: local files are untouched and the raw commands are an escape hatch. The gaps are in the live-remote steps: the Phase 7 repair has no freshness check and no backup of the remote, and baselining the read-back hides drift.

**Strengths**: local files are byte-identical; the output is bounded to inserted backslashes; staged gates; the raw commands remain as a recovery path; capture drift stops the phase; the repair is manual, one item at a time.

**Findings**:
- major/high, Phase 7 §4: The repair push can overwrite unpulled remote edits.
- major/medium, Phase 7 §2/4: No backup of the remote description.
- major/high, Migration Notes / Phase 6: Baselining the read-back hides escaper defects and drift.
- minor/medium, Phase 6: No runtime fallback if the escaper changes meaning.
- minor/medium, Phases 2–3: The capture target is inconsistent.

### Usability

**Summary**: The surface is small and escaping is invisible to `work`/`sync` users. The friction is in the help text, the skill docs, the error message, the placement of the exit code, the divergence between the raw and escaped commands, and the drift signal the plan relies on.

**Strengths**: zero-configuration escaping on every push path; `--body-file` mirrors the existing flags; the clap-level conflict; a discoverable API next to `markdown::github`; the repair uses an ordinary command.

**Findings**:
- major/medium, Migration Notes: The drift-as-conflict claim contradicts the work item.
- minor/high, Phase 1: Raw `--body-file` silently skips escaping.
- minor/high, Phase 1 §4: The docs target is vague and the table does not exist.
- minor/high, Phase 1 §4: The exit code has no value and lands in the legacy fixture.
- minor/medium, Phase 1 §3: The error omits the path.
- minor/medium, Phase 1: The stated reason for `--body-file` does not match the capture steps.
- suggestion/medium, Phase 7: The detection steps give no commands.
- suggestion/low, Migration Notes: No snapshot refresh procedure.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-11-23T22:50:43+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Architecture**: `linear create <work-item-file>` pushes unescaped — Resolved
- 🟡 **Correctness + Test Coverage + Code Quality**: Simulator cannot reproduce `X11`/`X14` — Resolved
- 🟡 **Architecture + Code Quality**: Duplicated option set — Resolved
- 🟡 **Correctness**: `<span>crates.io</span>` contradiction — Resolved
- 🟡 **Correctness**: Digit rule inside multi-line inline HTML — Resolved
- 🟡 **Correctness**: Capture test assumes one link form — Resolved
- 🟡 **Test Coverage**: `assert_escapes` lacks equivalence — Resolved
- 🟡 **Test Coverage**: No multibyte case — Resolved
- 🟡 **Safety + Usability**: Read-back baselining hides drift — Partially resolved (Phase 7 added; its design has new gaps)
- 🟡 **Safety**: Repair overwrites unpulled edits — Resolved
- 🟡 **Safety**: No remote copy before repair — Partially resolved (ephemeral `mktemp`, unchecked reads)
- 🟡 **Compatibility + Usability**: `UPDATE_NO_FILE` unnumbered — Resolved
- 🔵 **Safety**: Capture target inconsistent — Partially resolved (now consistent, but the mechanism is wrong)
- 🔵 **Test Coverage**: Escaped-versus-linked oracle — Partially resolved (`X19` mailto branch open)
- 🔵 **Test Coverage**: Jira substring check — Partially resolved
- 🔵 **Architecture**: Simulator seam — Partially resolved (convergence has no owner)
- 🔵 **Usability**: Phase 8 commands — Partially resolved
- Every other minor finding and suggestion — Resolved

### New Issues Introduced

- 🟡 **Architecture + Code Quality + Correctness**: Phase 7 verdict is mis-specified. `ReadBack` collides with `work::promotion::ReadBack`. `of_push` takes a rendered body, but the sites hold content and a projected remote, so every push would warn. `RemoteUnrenderable` and dialect rejection have no variant. The cited sites point at pull (`apply.rs:1087`), the shared `record`, and promotion's constructor, and miss `creation.rs:851`. Promotion has no report channel and already judges the read-back.
- 🟡 **Usability + Code Quality + Compatibility**: Warning reporting. A `RunFailure` renders as "push failed" for a push that succeeded and needs a `FailureClass`. `ReportedItem.notice` fits better. `W_PUSH_REWRITTEN` breaks the `warning:` stderr convention that skills match. The skill rows to change (create-work-item, extract-work-items, update-work-item) are not named.
- 🟡 **Usability**: The warning's remedy ("re-push or capture") cannot be acted on by whoever sees it, since a re-push is deterministic.
- 🟡 **Test Coverage + Code Quality**: The whole-body fallback masks rule over-reach. Every unchanged case passes even if a rule escapes inside code or an attribute.
- 🟡 **Test Coverage + Compatibility**: No test shows an ordinary Jira or Shortcut push stays silent.
- 🟡 **Correctness + Architecture**: Promotion's retitle sends the body (`promotion.rs:735-739`). The Phase 6 assertion "carries no `description`" is wrong.
- 🟡 **Safety**: `linear.team_key` does not choose the create team. `resolve_team` reads `linear.team_id`, then the catalogue, so captures would land in PP.
- 🟡 **Compatibility**: No CHANGELOG `Breaking` entry for raw-command escaping, and none for `--body-file`, `--verbatim`, 115 or the warning.
- 🔵 **Code Quality + Compatibility + Correctness**: `Description` lacks `PartialEq, Eq`, which `UpdateFields` derives.
- 🔵 **Compatibility**: `linear-client` has no public-API snapshot. `tests/discriminant.rs` is an unlisted caller.
- 🔵 **Correctness**: `X10` already passes the current simulator. Only `X12` and `X15` drive the change.
- 🔵 **Correctness**: Promotion created and then retitled pushes twice, so "one warning" is wrong.
- 🔵 **Correctness**: The rules escape constructs no capture covers (inline HTML, bold, heading, table cell, blockquote); quoted URLs are not exempt.
- 🔵 **Usability**: `--verbatim` is guarded on update but not on create. The `--body` parity claim is overstated.
- 🔵 **Safety**: The Phase 8 pre-repair sync should be `--preview`. There is no post-repair audit. The warning fires only once.
- 🔵 **Code Quality**: The description source is modelled in two clap mechanisms. The `escape_for_push` doc references another crate's behaviour.
- 🔵 **Architecture**: `reads_alike` is a third notion of "same reading" beside PP-880's canonical form.

### Assessment

The escaper core is now sound: every finding against Phases 1–5 is resolved or reduced to a minor. What remains sits mostly in Phase 7, which the first pass added, and in wiring details (retitle, capture team, derives). Phase 7 needs a proper design pass. That means a non-colliding verdict type built on `BodyComparison::equivalence` over content and a projected remote, the real push sites, an item notice rather than a `RunFailure`, one `warning:`-shaped token, and actionable wording. The remaining fixes are mechanical.

## Re-Review (Pass 3) — 2026-11-23T23:16:36+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Architecture + Code Quality + Correctness**: Phase 7 verdict mis-specified — Partially resolved (the name, inputs, `Unjudged` and sites are fixed; the promotion rule and the rejected-body mapping remain)
- 🟡 **Usability + Code Quality + Compatibility**: Warning reporting — Resolved
- 🟡 **Usability**: Warning remedy not actionable — Partially resolved (no destination, no safe way to clear the mark)
- 🟡 **Test Coverage + Code Quality**: Fallback masks rule over-reach — Resolved
- 🟡 **Test Coverage + Compatibility**: No silent Jira/Shortcut test — Resolved
- 🟡 **Correctness + Architecture**: Retitle sends the body — Resolved
- 🟡 **Safety**: Capture team mechanism — Resolved
- 🟡 **Compatibility**: CHANGELOG — Resolved
- 🔵 **Code Quality + Compatibility + Correctness**: `Description` derives — Resolved
- 🔵 **Compatibility**: public-API snapshot and callers — Resolved
- 🔵 **Correctness**: `X10` red step — Resolved
- 🔵 **Correctness**: Uncovered constructs and quoted URLs — Resolved (`X21`-`X25`)
- 🔵 **Usability**: `--verbatim` guard and `--body` parity — Resolved
- 🔵 **Safety**: Phase 9 pre-checks, re-audit, one-shot warning — Resolved
- 🔵 **Test Coverage**: Jira ADF equality — Partially resolved (blind to escapes, because ADF unescapes)
- 🔵 **Architecture**: `reads_alike` as a third notion of equivalence — Still present (suggestion)

### New Issues Introduced

- 🟡 **Correctness + Architecture + Test Coverage + Code Quality + Compatibility**: The rules for keeping the mark don't match how baselines are written. "Every non-push site records `None`" would clear it on `RecordBaseline` (frontmatter-only or fetched syncs, `classify.rs:388-389`), on realign (`realign.rs:234-252`) and on `without_local_digests` (`baseline.rs:158-172`). `work create --push` and promotion write through `record_created_baseline`/`IntendedBaseline` (`created_baseline.rs:33-40`), which can't carry it. `Entry::Legacy` can't hold it.
- 🟡 **Architecture + Correctness**: Promotion's "final read-back" can be a human edit (`Retitling::Keep(Edited)`), no push at all (`UserNamedAdopt`), a push that didn't land (`UpdateFailed`), or a fresh `show` on resume.
- 🟡 **Safety + Correctness**: The Phase 9 audit finds nothing. `linear show` nests the description at `.data.issue.description`, and the lone-number regex can't match Linear's `  160. ` shape.
- 🟡 **Safety**: Section headings written into the hand-rendered baseline JSON need escaping through `render_string`. An unparseable baseline reads as empty (`baseline.rs:386-406`), which would drop every item.
- 🟡 **Usability**: The `note:` remedy names no destination and no step the user can take today. A pull clears the mark only by accepting the corruption.
- 🟡 **Test Coverage + Code Quality**: The repository sweep in `work-adapters` can't call the crate-private `proposed_escapes`.
- 🔵 **Architecture + Code Quality + Compatibility + Correctness**: A single `Option<ItemNotice>` slot collides with `NothingToSettle` on `--resolve create`.
- 🔵 **Correctness + Code Quality**: A rejected local body reads as `Different` through `equivalence`, so it can't map to `Unjudged`.
- 🔵 **Correctness**: The `X19` Phase 2 gate and the Phase 5 oracle disagree about the mailto case.
- 🔵 **Architecture + Code Quality**: The creation and promotion `read_back_of` helpers discard the projection. The policy for keeping the mark is repeated at five sites. `push_fidelity` would sit better on `BodyComparison`.
- 🔵 **Test Coverage**: No test separates promotion's two read-backs or a resumed promotion. A marked item re-pushed in the same run isn't covered. Scheme-URL boundary rows are missing. The Jira ADF equality check can't see escapes.
- 🔵 **Usability**: The `note:` line has no `<id>:` prefix. One condition goes by three names. The step 1 count can't reconcile. Step 4's remote file has no command.
- 🔵 **Compatibility**: The committed `last-sync.json` means one sync from an older build drops every mark. The list label golden and the sync skill's structure test aren't updated. The `golden_digests` criterion doesn't guard baseline bytes.
- 🔵 **Code Quality**: The arity of the `CreateArgs` group is unstated. Doc comments in the plan's code blocks break the comment policy. One code fence is broken.
- 🔵 **Safety**: The pinned team id isn't checked against `TES` before the create, and the restore isn't required on every exit.

### Assessment

Phases 1–6 are ready to implement: nothing major remains against the escaper, the CLI, the captures or the wiring. Every major in this pass sits in Phases 7–9, the read-back notice and its persistent baseline mark that the first two passes added. Each pass has found new structural gaps there. They touch the baseline schema, the promotion state machine and the sync classifier, all well outside Linear escaping. That pattern suggests the notice and mark belong in their own work item with their own research and plan, with only the transient notice kept here, or none at all.

## Re-Review (Pass 4) — 2026-11-23T23:30:15+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Five lenses**: Rules for keeping the mark don't match how baselines are written — Partially resolved (the three-group table is accurate, but `RecordBaseline` after an equivalent body comparison keeps a mark it should clear)
- 🟡 **Architecture + Correctness**: Promotion's final read-back can be a human edit — Partially resolved (the `Unjudged` routes are right; the verdict has no path to `finish_promotion`, and the create clause can never apply)
- 🟡 **Safety + Correctness**: The Phase 9 audit finds nothing — Partially resolved (the path and patterns are right; the self-check ORs the two patterns)
- 🟡 **Safety**: Heading escaping — Resolved
- 🟡 **Usability**: Remedy not actionable — Partially resolved (it is now actionable, but the changed text is never shown)
- 🟡 **Test Coverage + Code Quality**: The sweep can't reach `proposed_escapes` — Resolved
- 🔵 Every pass-3 minor finding — Resolved, apart from those carried into the list below

### New Issues Introduced

- 🟡 **Correctness**: `RecordBaseline` reached after the classifier finds the bodies `Equivalent` (`classify.rs:370-374`, `:462-463`) keeps the mark, so a remote fixed by hand stays marked indefinitely.
- 🟡 **Architecture + Correctness**: The promotion verdict has no stated route through `promote`'s stage loop to `finish_promotion` (`promotion.rs:398-448`, `:867-921`), which also has two callers with no verdict. The "create" row cites the wrong site (the create's read is at `:573-578`), and its clause can never apply.
- 🟡 **Usability**: The clearing note says to backtick "the changed text", but only headings are printed or persisted. `SectionDiff` already carries both sides.
- 🔵 **Correctness**: `ReuseId` creates (`apply.rs:1279-1281`, `creation.rs:572-578`) judge a read-back that follows no push.
- 🔵 **Correctness**: Sync's title-only retitle is judged as if it pushed the body.
- 🔵 **Correctness + Test Coverage**: A marked item whose push is `Unjudged` prints no notice. The notice should come from the mark after `after`.
- 🔵 **Safety + Correctness**: The audit self-check passes when only one of its two patterns matches.
- 🔵 **Safety + Usability**: `exit 1` in pasted snippets closes the operator's shell. A null description stops the audit. Remote edits made between the step 4 check and the step 5 push can be overwritten.
- 🔵 **Architecture**: The renderer moves CLI text into `work-adapters`, outside `report_records::single_line`.
- 🔵 **Architecture + Code Quality**: Clearing is the default for the mark. A required `MarkChange` at the `set_baseline` funnel would make every site choose.
- 🔵 **Code Quality**: `RewrittenOnPush` names both the notice variant and the mark. `rejects_local` doesn't need to be public. `lines()` can't fill in `<path>`.
- 🔵 **Test Coverage**: Only one of the four clearing sites is tested. The keep-on-`Unjudged` case only applies to update and retitle. Legacy entries have no test. The sweep needs a floor on how many bodies it reads. The filters miss named tests.
- 🔵 **Usability**: The tests still say "two lines". There is no way to acknowledge a mark.
- 🔵 **Compatibility**: The older-build caveat belongs under CHANGELOG **Migrations**. Keeping unknown split keys would stop the next added key being lost the same way.

### Assessment

The plan is converging. Three majors remain, down from six. Four of the seven lenses (test coverage, code quality, compatibility and safety) found no major, and Phases 1–6 and 9 have only minor findings. The three majors are specification gaps in Phases 7–8: one rule for the mark, one plumbing path, and one piece of output content. None of them is a redesign.

## Re-Review (Pass 5) — 2026-11-23T23:41:42+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness**: `RecordBaseline` after an equivalent comparison keeps the mark — Partially resolved (the rule is right; the flag never reaches the code that applies it, and `SendFields` is missed)
- 🟡 **Architecture + Correctness**: Promotion verdict path — Resolved (minor follow-ups on `retire` and on `NotPromoted` rows)
- 🟡 **Usability**: Changed text never shown — Resolved
- 🔵 Every other pass-4 minor finding — Resolved, apart from those carried below

### New Issues Introduced

- 🟡 **Architecture + Code Quality + Correctness**: `bodies_agreed: bool` on `RecordOnly` is a bare flag that never reaches the code that applies it: `decide` reduces it to the unit `Action::RecordBaseline` (`decide.rs:113-122`), and `PlannedAction` carries only state and action (`plan.rs:66-71`). It also misses `SendFields` after an equivalent comparison (`classify.rs:398-399`), and changes `work`'s public API without a regeneration step.
- 🟡 **Usability + Correctness**: `ChangedLine` pairs raw source lines inside a differing section. Cosmetic differences such as bullet markers, code-span indentation and kept escapes are reported as changes of meaning and can crowd out the real rewrite. How lines pair when the two sides differ in line count is undefined.
- 🟡 **Safety + Correctness**: The Phase 9 self-check requires exactly one match per pattern in `read-back.json`, but Phase 2 retakes that file with `X10`-`X25`, so the check can never pass.
- 🔵 **Code Quality + Architecture + Correctness**: `work update --push` writes through `BaselineStore::set` directly (`update.rs:256-270`), bypassing `MarkChange`.
- 🔵 **Compatibility + Architecture + Correctness + Test Coverage**: Unknown split keys survive only on entries this build doesn't rewrite. Rendering them after the known keys breaks the alphabetical key order. An extra field in a mark object counts as malformed.
- 🔵 **Correctness**: An `Unjudged` re-create (`create_anew`) inherits the previous issue's mark.
- 🔵 **Safety + Correctness**: The lone-number pattern misses a stray list inside a blockquote (`X24`). A not-found issue blocks step 1 indefinitely. The subshells depend on `$backup` already being set.
- 🔵 **Safety**: The persisted mark copies unbounded remote text into the committed `last-sync.json`.
- 🔵 **Architecture**: `Promoting` is `&self`-only, and `retire` must forward the verdict. `NotPromoted` rows need it. Sync's `ItemSubject.path` has no source in `ReportedItem`.
- 🔵 **Test Coverage**: The clearing and keeping tables don't tell the two routes into `reconcile_fields` apart. The "fetched" keep row can't start from a marked entry. The cap test misses its boundary. `RewriteMark::after` has no direct test.
- 🔵 **Usability**: There's no way to see changed lines beyond the first five. Step 2's subshell exit status is ambiguous. Step 5 isn't scripted.
- 🔵 **Code Quality**: Line pairing is an algorithm the plan never names (`similar` is already a dependency). `MarkChange::After` and `ItemSubject.shown` could carry more intent.

### Assessment

The plan is still converging: three majors again, but each narrower than the last pass's. One is a domain type for the body-agreement signal and its route to apply. One is filtering changed lines by meaning. One is scoping a Phase 9 self-check to named fragments. Phases 1–6 are stable, and the remaining minor findings sit mostly at the edges of Phase 8's baseline mark.

## Re-Review (Pass 6) — 2026-11-23T23:51:08+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Architecture + Code Quality + Correctness**: `bodies_agreed` never reaches apply — Partially resolved (`BodyAgreement` on `PlannedAction` is right; its route out of `classify` is unstated)
- 🟡 **Usability + Correctness**: `ChangedLine` reports cosmetic lines — Partially resolved (the meaning filter is right, but judging each hunk on its own drops the digit-rule rewrite; see below)
- 🟡 **Safety + Correctness**: Self-check pinned to the old capture — Resolved
- 🔵 Unknown keys, extra mark fields, `work update --push` funnel, blockquote pattern, not-found handling, self-contained `$backup` — Resolved
- 🔵 Re-create inherits the old mark — Partially resolved (the intent is stated, but `MarkChange` cannot express it)

### New Issues Introduced

Three of the four majors were introduced by the pass-5 fixes.

- 🟡 **Test Coverage + Correctness**: `changed_lines` judges each hunk on its own text. Out of context, `  0160.` and `  160.` both parse as list items, so the `X09` and `X24` rewrites are filtered out and the verdict carries no changed lines. The `X09` test also misdescribes the captured hunk: there is no inserted blank line, and the bullets change `-` → `*`.
- 🟡 **Usability + Safety + Correctness**: The Phase 9 step 4 block can print `repaired` after a failed or rewritten push. Without `pipefail`, `tee` masks the push's exit status, and a `!`-negated pipeline is exempt from `set -e`. Nothing checks that the preview reported no action.
- 🟡 **Usability + Safety**: Phase 9 step 5 re-audits the pre-repair backups, so it cannot confirm a repair. Refreshing the backups through step 1 would overwrite them.
- 🟡 **Code Quality + Architecture + Correctness**: The plan never says how `BodyAgreement` leaves `classify`, which returns a `Copy` `Classification` (`classify.rs:539-544`, public API at `public-api.txt:2009`).
- 🔵 **Code Quality + Architecture + Correctness + Compatibility**: Realign, `finalise_run` and rename write in memory and never through `BaselineStore::set`, so `record` cannot be their funnel. Making `set` private also breaks five test and `work-cli` callers.
- 🔵 **Architecture + Correctness**: `MarkChange` needs a `Fresh` variant for creates and relinks.
- 🔵 **Correctness**: The plan describes a `--resolve <id>=record-baseline` route that does not exist (`parse_orders`, `sync.rs:390-408`).
- 🔵 **Compatibility + Correctness**: The persisted mark's shape has nowhere to put `more`.
- 🔵 **Compatibility**: `PlannedAction`'s fields are private, so the expected API diff is wrong. It also has 6 unlisted `new` callers in `work-cli`.
- 🔵 **Code Quality**: `with_rewrite_mark` bypasses `record`. Bounding is specified in three places, so the first and later notices differ.
- 🔵 **Usability + Safety**: The blocks need bash; under zsh, `read -p` and word splitting break. The self-check fails silently. The widened lone-number pattern flags correctly escaped `)` continuations.
- 🔵 **Usability**: Truncated lines show no marker. The overflow pointer leads to raw JSON or ADF.
- 🔵 **Test Coverage**: Rendered-output assertions are placed in `work-adapters`, which cannot render them. The truncation test is ASCII-only. There is no assertion for the rewound retirement's mark. The Phase 1 filter misses `flow_create`.

### Assessment

The pass count has stopped falling: 4 majors, against 3 in pass 5. Three of the four are regressions from the pass-5 fixes. Two are shell errors in Phase 9's operator snippets, and one is the hunk filter I specified. Phases 1–6 are unchanged and clean. Every major sits in Phase 7's line reporting, Phase 8's classifier plumbing or Phase 9's runbook. Specifying these mechanisms in prose has stopped converging. The remaining issues would be found faster by writing the code test-first than by another review pass.

## Re-Review (Pass 7) — 2026-11-24T00:01:40+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Test Coverage + Correctness**: Hunks judged out of context — Resolved
- 🟡 **Usability + Safety + Correctness**: Step 4 false `repaired` — Resolved
- 🟡 **Usability + Safety**: Step 5 re-audits stale backups — Resolved
- 🟡 **Code Quality + Architecture + Correctness**: `BodyAgreement` route out of `classify` — Resolved
- 🔵 Every pass-6 minor — Resolved, except the encapsulation follow-up below

### New Issues Introduced

- 🟡 **Code Quality** (minor from **Architecture** and **Compatibility**): `..entry.clone()` on an enum variant does not compile (E0436). `Entry::Split`'s fields are public, and `BaselineStore::set` still replaces whole entries, so "only the baseline module sets the mark" is a convention rather than a guarantee.
- 🟡 **Usability + Safety** (minor from **Correctness**): Items fetched into `before-retry-N/` are never audited, and step 4 refuses them for "no backup". Step 5 passes even when its own fetch failed reads.
- 🟡 **Correctness** (minor from **Safety**): Step 5's `rg` over several `.out` files puts the file name in front of each id. With two or more repairs, every re-fetch then fails.
- 🔵 **Compatibility + Safety + Correctness**: A malformed mark kept raw in `unknown` collides with a newly set mark, and a `Clear` leaves it in place.
- 🔵 **Compatibility + Architecture**: Two `classify` callers are unlisted (`work/src/promotion.rs:148-162`, `work-adapters/tests/sync_recipe_migration.rs:745`).
- 🔵 **Architecture**: The route from `PlannedAction` into `RecordRequest`/`RetitleRequest` and `set_baseline` is unstated. Push and retitle share `record_read_back`.
- 🔵 **Architecture + Code Quality**: Per-line mark extras have no home. Use `MarkedLine { line, extra }`.
- 🔵 **Code Quality**: `NotCompared` misnames routes that did compare and found a difference. `recover` returns `Agreed` only on its `RecordOnly` branch. `RewrittenLines::from` is lossy, so name it `bounded`.
- 🔵 **Test Coverage**: The two `X09` tests contradict each other. Phase 7 has no test of its own for the 200-byte bound. No multi-line hunk is rendered. The no-surviving-hunk input is unnamed. `unknown` is checked on only one whole-document rewrite.
- 🔵 **Correctness**: A rewritten level-2 heading is reported as a deleted section plus an inserted one. The splice check must read both sides the same way under Jira's dialect. `pipefail` with `rg -q` can skip a large description.
- 🔵 **Safety**: The `updatedAt` check runs before the human review, not just before the push. A missing fragment passes the faithful self-checks.
- 🔵 **Usability**: `audit ok` prints even when ids are listed. Several commands in step 4 exit without saying why. The id is typed twice. The step 7 relative paths break. The overflow note names a local id, not the tracker's.

### Assessment

Three majors again, each narrow, and each a slip in a concrete artefact: one compile error and two shell-script bugs. Phases 1–6 have been clean for four passes. The design of Phases 7–8 is settled; the open items are details. Spelling Phase 9's runbook and Phase 8's Rust out in prose keeps producing defects that running the code would catch at once. The scripts should be committed as files and exercised against fixtures, and the Rust written test-first. That would retire this class of finding faster than further review passes.

## Re-Review (Pass 8) — 2026-11-24T00:15:08+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Code Quality**: `..entry.clone()` does not compile; mark encapsulation only by convention — Resolved (`Entry::Split(SplitEntry)` with private fields; `seed` behind `test-support`). Erasure through the public `Baseline::set` plus `replace` remains (minor, below).
- 🟡 **Usability + Safety**: Retry directories never audited — Partially resolved (they now merge into `before/`, and the audit refuses unread items; the loop itself is fragile, see below)
- 🟡 **Correctness**: Step 5 `rg` file-name prefix — Resolved
- 🔵 Every pass-7 minor — Resolved

### New Issues Introduced

- 🟡 **Usability + Safety** (minor from **Correctness**): The step 1 retry loop runs inline, without a bound, and is hard-wired to `before/` and `retry-$n`. Steps 5 and 7 tell the operator to adapt it, which can overwrite `after/failed-reads.txt` with a stale list and let `--expect-none` pass with unread items. Under zsh, the unmatched glob aborts the loop. Fix: make it a bounded `refetch.sh <dir>`, and have `audit.sh` check that saved + not-found equals the id count.
- 🔵 **Correctness**: The stray-list verdict catches `X09` only because Linear drops the leading zero. A wrapped `42.` stored as a list reads alike under CommonMark, so Phase 7 would return `Preserved`. Either state the limit or add a Linear-specific check.
- 🔵 **Correctness**: The splice check reads both sides raw, but sections are selected through `as_pushed`. Under Jira, a hunk the ADF round trip absorbs can still be reported.
- 🔵 **Architecture + Compatibility + Correctness**: `Baseline::set` plus `BaselineStore::replace` stays public, so a mark can still be erased. `--all-features` in CI compiles `seed` in, and no guard like `config_test_support.py` exists.
- 🔵 **Compatibility + Code Quality + Architecture**: The `Entry::Split` match and construction sites in `work-adapters` are under-listed (`fetch.rs:373`, `run.rs:684`, `send_refusal.rs:24`, `realign.rs:234`, tests). `unknown` must exclude the known keys that `render_split` writes.
- 🔵 **Code Quality + Architecture**: Request structs accept `MarkChange` variants that cannot occur there; take `BodyAgreement` instead. A malformed mark belongs in a `MarkState { Absent, Marked, Unreadable }`. `advanced_to` and `renamed` carry nothing. `RetitleRequest` is built in `send_outcome`, not `record_outcome`.
- 🔵 **Test Coverage + Code Quality**: The fallback test's dialect hook cannot reach a check that never calls `as_pushed`; use a cosmetic-only hunk under `PlainDialect`. The property test cannot fail while the fallback exists; assert that the filter alone keeps a hunk. The heading test should go through `push_fidelity`.
- 🔵 **Safety + Usability**: The post-repair sync is real, and accepts any action; use `--preview` with an exact `noop` match. Re-running `repair.sh` overwrites its `.out`. A failed audit leaves an empty `affected.txt`. `repair.sh` reports a failed read as an edit, and does not validate its argument. `X09` is missing from the faithful stray-list codes.

### Assessment

The plan has converged to COMMENT. One major remains, which a bounded `refetch.sh` resolves, and every other finding is minor. Majors per pass: 12, 8, 6, 3, 3, 4, 3, 1. Phases 1–6 have been clean for five passes. The design of Phases 7–8 is settled, and the remaining minors are implementation details that test-first work will meet in any case. The one minor with design weight is the stray-list verdict's dependence on Linear dropping a leading zero. It sets the limit of what Phase 7 can detect under PP-880's CommonMark reading, and the plan should state it.

## Changes After Pass 8 — 2026-11-24T00:18:15+00:00

Every pass-8 finding was applied to the plan without a further review pass:

- **Retry loop.** A bounded `refetch.sh` (5 retries with a pause, named `<dir>.retry-<n>` directories, a non-zero exit while reads still fail) replaces the inline loop. Steps 1, 5 and 7 each call it. `audit.sh` takes the fetch's `ids.txt` and refuses a directory whose saved and not-found counts do not add up. Dry-run against a stub `accelerator`: a flaky read recovers, a persistent failure stops after 5 retries.
- **Leading-zero limit.** Stated in the Desired End State, and pinned by `a_stray_list_without_a_leading_zero_reads_alike`.
- **Splice check.** Reads both sides through `as_pushed`, the reading that selected the section.
- **Fallback and heading tests.** The fallback test uses a cosmetic-only hunk under `PlainDialect`, checked with `meaningful_hunks`. `the_filter_alone_finds_every_captured_rewrite` replaces the property test that could not fail. The heading test runs through `push_fidelity`.
- **Mark storage.** `MarkState { Absent, Marked, Unreadable }` holds the mark. `unknown` excludes the fixed key set. `advanced_to` and `renamed` are dropped.
- **Write access.** `Baseline::set` and `replace` become `pub(crate)`. `seed` sits behind a `test-support` feature guarded by a generalised `config_test_support.py`, with a featureless `cargo check` among the criteria.
- **Site list and requests.** Every `Entry::Split` site is listed. The requests carry `BodyAgreement`, and the `send_outcome` citation is corrected.
- **`repair.sh`.** Validates its argument, refuses an id already repaired, appends to its log, reports a failed read as a failed read, and runs the post-repair check as a `--preview` with an exact `noop` match.
- **`audit.sh` and the self-check.** `affected.txt` is written only when the audit passes. `X09` joins the faithful self-check codes.

## Final Verdict — 2026-11-24T00:19:49+00:00

**Verdict:** APPROVE

Approved for implementation after the post-pass-8 changes.
