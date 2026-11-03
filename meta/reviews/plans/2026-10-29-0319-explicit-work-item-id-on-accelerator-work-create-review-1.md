---
type: "plan-review"
id: "2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create-review-1"
title: "Plan Review: Explicit Work Item ID on accelerator work create Implementation Plan"
date: "2026-10-29T16:24:18+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "usability", "compatibility", "security", "standards"]
review_number: 1
review_pass: 7
tags: ["work", "cli", "allocation", "create-work-item"]
last_updated: "2026-10-31T14:40:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Explicit Work Item ID on accelerator work create Implementation Plan

**Verdict:** REVISE

The plan is sound at its core. The collision check sits at the existing
lock-held `allocate` seam ahead of the push, so check-then-write is atomic
and a refusal sends nothing. Admission runs before `create_dir_all`, the
domain model lives in the regex-free `work` crate, and every acceptance
criterion traces to a named test. No lens found a critical issue. It needs
revising for eight major findings in four areas:

- the pattern grammar and the held-ID equivalence rule would each exist in
  more places than before;
- the skill would do deterministic ID classification in prose, against
  ADR-0045, without saying so;
- other readers of on-disk IDs mishandle the shapes `--id` now writes;
- several tests could not fail on the property they claim to pin.

### Cross-Cutting Themes

- **Pattern grammar and ID equivalence keep multiplying** (flagged by:
  architecture, code-quality, correctness, test-coverage) — Phase 2 adds an
  exact walk, a lenient walk and a `Segment::Number { width }` parser beside
  `match_full_id`, `pattern_cap`/`explicit_width` and `canonical_digit_width`.
  The key-and-number equivalence rule stays private to `RequestedId`, while
  `identity.rs` and `rekey::sequence` keep their own rules. The
  "refactor first" step also changes `work resolve` for non-`0Nd` number
  specs: today `segments` maps any `number:` token to a number, which was
  verified at `cli/work/src/resolve.rs:93`. No `resolve` test pins that
  branch.
- **The taken check trusts a scanner the caller builds** (flagged by:
  architecture, code-quality, usability, test-coverage) —
  `HeldIds::scanner_for_requested_key` must be built for the requested key,
  ignoring case, and only the field name says so. The Phase 3 filename-holder
  fixtures don't say whether the files carry a matching frontmatter `id`. If
  they do, the frontmatter comparison masks a wrongly keyed or case-sensitive
  scanner.
- **Dry-run parity is duplicated and overstated** (flagged by: code-quality,
  architecture, correctness) — `refuse_requested_id_before_preview` repeats
  the opening of `creation_context`. It also runs the collision check earlier
  than the real create does, before author, template and `integration_state`.
  "Refuses exactly as the real create does" therefore holds only for the five
  `--id` refusals.
- **Refusal text and codes** (flagged by: usability, code-quality,
  standards, test-coverage) — `E_ID_TAKEN` names neither the field nor the
  held value, although the taken rule matches `400` against `0400` and
  `PP-905` against `PP-0905`. Variant names drift from their codes
  (`AlongsideProject` against `E_ID_WITH_PROJECT`). The distinct-code tests
  build their sets from literals, so they cannot fail on distinctness.
- **The skill's edges** (flagged by: architecture, security, correctness,
  compatibility, usability, test-coverage) — classification lives in prose.
  The ID goes into Bash unquoted. A correction can produce `0000`. A topic
  that mentions `--id` in prose gets parsed as a flag. A non-`E_ID_` exit 1
  from the `--id` dry-run is misread as a tracker failure. No eval covers a
  refusal from the pushed create at step 7.3.
- **What `--id` writes against what the corpus can read** (flagged by:
  compatibility, architecture, security) — `normalise_id` accepts only a
  letters-only prefix, which was verified at
  `cli/corpus/src/work_item_id.rs:162-171`. `is_canonical_id_token` is
  case-sensitive on `work.key`. The visualiser's scan regex is
  case-sensitive. Pattern literals are checked for path-hostile characters
  only when `compile_scan_regex` runs in check 5.

### Tradeoff Analysis

- **ADR-0045 against no new CLI surface**: Architecture wants the exact /
  one-correction / no-exact-form classification in the CLI, where it can be
  tested. The plan chose prose to avoid new CLI surface, with the CLI's
  mismatch refusal as the backstop. The backstop catches a wrong proposal,
  but not a valid ID the skill wrongly rejects. At minimum, record the
  departure and its cost in the Decisions table. A cheap middle path is for
  the skill to check its proposed form against `work canonicalise-id` output,
  or to defer CLI validation to 0371's validate-only `--dry-run` as a named
  follow-up.
- **Align other ID readers now or defer them**: Compatibility offers widening
  `normalise_id` and `is_canonical_id_token` to the taken rule's key grammar.
  That moves this story into `corpus` and `work sync`. Recording a non-goal,
  pinning current behaviour with a test, and raising a follow-up keeps 0319
  bounded. The deferral is the recommended route.
- **Composition matrix breadth against speed**: Test Coverage suggests
  cutting Phase 3's ~50 end-to-end refusal runs to one per class plus the
  wiring-sensitive cases. The full matrix buys traceability against each
  acceptance criterion. Keep the full matrix in `work::requested_id` and only
  the wiring cases in `create.rs`; the traceability table can name the domain
  test.
- **`E_PATTERN_OVERFLOW` wording**: Usability points out that the unchanged
  message ("Archive completed work items or widen the pattern") is the wrong
  remedy after a slip to `--id 9999`. The plan deliberately leaves it
  unchanged and amends the work item instead. Naming `highest_file` in the
  at-cap branch is a one-line change; weigh it against the decision.

### Findings

#### Major

- 🟡 **Architecture**: Deterministic ID classification sits in skill prose
  against ADR-0045, and the plan doesn't acknowledge it
  **Location**: Phase 5 §2 step 3; Decisions "Skill correction"
  The decision table pads numbers, adds `work.key` and judges exact shape,
  which is exact procedural logic that ADR-0045 assigns to the CLI. The skill
  now holds a third copy of the pattern grammar, checked only by evals at a
  0.9 mean pass rate.
- 🟡 **Architecture + Code Quality**: The key-and-number equivalence rule is
  private to `RequestedId`
  **Location**: Phase 2 §3 (`refuse_if_taken`)
  `identity::{holder_of, …}` compare whole strings ignoring case, and
  `rekey::sequence` splits on a hard-coded `-`. The work item says 0302
  should converge on this rule, which means pulling it out of `RequestedId`
  first. Name it as a domain concept, for example
  `parse_held(pattern, text) -> Option<HeldKeyNumber>`, and state how it
  relates to `rekey::sequence`.
- 🟡 **Code Quality + Architecture**: The pattern grammar is walked three
  times, and the width is parsed twice, in the `work` crate
  **Location**: Phase 2 §1, §3
  The exact walk and the lenient walk each re-implement the key run, the
  digit run and literal matching from `match_full_id`.
  `Segment::Number { width }` sits beside `next_number::pattern_cap`. Move
  the walking primitives into `id_pattern.rs` and derive `pattern_cap` from
  `segments`.
- 🟡 **Compatibility + Architecture**: Existing ID readers reject or
  mishandle the shapes `--id` writes
  **Location**: Decisions "Case-insensitive scan"; What We're NOT Doing
  `X1-0400` gets no `work_item_id` in the visualiser's indexer. `xy-0400-…`
  and `pp-0500-…` get no slug from its case-sensitive scan.
  `is_canonical_id_token` makes `work sync` skip conflict dossiers for these
  items. Either align the readers, or record a non-goal with a follow-up and
  pin tests.
- 🟡 **Usability + Code Quality**: `E_ID_TAKEN` names the holder's path but
  not the field or value that holds the ID
  **Location**: Phase 2 §3 (`RequestedIdRefusal::Taken`)
  Because matching is broad, the named holder often doesn't visibly carry the
  ID. `RetirementRefusal::IdTaken { holder, field }` already names the field
  and renders a repository-relative path. Because `work`'s API is pinned,
  adding this later is an API change.
- 🟡 **Test Coverage**: `meta_tree` cannot detect the empty `meta/work/`
  that admission's placement prevents
  **Location**: Phase 3 §1 (Refusing); Phase 4 §1
  `meta_tree` records only files; this was verified at
  `cli/work-cli/src/test_support.rs:201-224`. Admission placed after
  `create_dir_all` would pass every planned test. Add
  `assert!(!repo.path("meta/work").exists())`.
- 🟡 **Test Coverage**: The filename-scan wiring may be masked by
  frontmatter matches
  **Location**: Phase 3 §1 (`a_taken_id_is_refused_naming_its_holder`)
  In Phase 2 the scanner is a fake, so only Phase 3 proves that
  `scanner_for(.., requested.key())` and `compile_ignoring_case` are wired
  correctly. State that the filename holders carry no frontmatter `id`, or an
  unrelated one, at least for `xy-0400-other.md` against `XY-0400` and for
  `00400-x.md`.
- 🟡 **Test Coverage**: The eval gate lets new evals fail outright, and the
  red runs are discarded
  **Location**: Phase 5 §1, §3
  At a ≥ 0.9 mean over 50 evals, up to five of the twelve new evals could
  fail completely. Require `pass_rate` 1.0 for each of evals 39 to 50, and
  record the red runs under `old_skill`, as `benchmark.json` already does
  for 18 evals.

#### Minor

- 🔵 **Architecture + Code Quality + Usability**: The taken rule depends on
  an unenforced convention for the injected scanner
  **Location**: Phase 2 §3 (`HeldIds`); Phase 3 §4
  A scanner built for `work.key`, or a case-sensitive one, compiles and
  silently misses holders. Inject a key-taking factory or a domain port
  (`KeySequenceScanners::for_key`) so the domain chooses the key.
- 🔵 **Code Quality + Architecture + Correctness**: The dry-run repeats
  `creation_context`'s opening steps and orders the checks differently
  **Location**: Phase 4 §2
  Extract one preflight helper that both paths call. Narrow the parity claim
  and its traceability row to the five `--id` refusals until 0371.
- 🔵 **Code Quality + Correctness + Architecture + Test Coverage**: The
  "refactor first" step changes behaviour and leaves malformed patterns
  unspecified
  **Location**: Phase 2 §1
  Non-`0Nd` specs collapse to an empty list, so `work resolve` changes, and
  `admit` on an empty list could admit `""`. `{number:4d}` would surface as
  `E_ID_PATTERN_MISMATCH` rather than the pattern error. Keep the move pure,
  add `resolve` pins under `{number}` and `{key}-{number:06d}`, and validate
  the pattern through `compile_scan_regex` before admission.
- 🔵 **Security**: Path safety of `--id` rests on an unstated, partly
  incidental invariant
  **Location**: Phase 2 §3 (exact walk); Phase 2 §2
  Specify `is_ascii_*` predicates and a non-panicking number parse. Add
  traversal, separator, control-character and non-ASCII-digit inputs to the
  mismatch matrix. Make pattern-literal validation part of admission rather
  than a side effect of check 5.
- 🔵 **Security**: The skill puts the requested ID into Bash unquoted
  **Location**: Phase 5 §2 step 5
  The CLI cannot backstop shell parsing. Write `--id '<ID>'`, put only an
  Exact or user-confirmed ID into a command, and refuse an ID that contains
  `'`.
- 🔵 **Correctness**: The case-insensitive literal comparison in the lenient
  parse must respect character boundaries
  **Location**: Phase 2 §3 (`refuse_if_taken`)
  `rest[..lit.len()]` panics on a short remainder, or mid-character on a
  pasted en dash. Use `rest.get(..n).is_some_and(..)` and add a non-ASCII
  held-ID case.
- 🔵 **Correctness**: A skill correction can produce an ID the CLI refuses
  **Location**: Phase 5 §2 step 3
  Left-padding `0` gives `0000`. A correction should apply only when its
  result meets every Exact condition. Add `"0"` to eval 46.
- 🔵 **Correctness**: Non-`E_ID_` exit-1 refusals from the `--id` dry-run
  are misread as tracker failures
  **Location**: Phase 5 §2 step 5 (7.1)
  `E_WORK_KEY_REQUIRED` and similar errors fall through to "save locally".
  Add a branch that relays stderr and aborts.
- 🔵 **Architecture + Test Coverage**: Pending-push retries under `--id` are
  not designed
  **Location**: Phase 3 §4; Phase 3 Push tests
  After `loud-terminal`, a same-`--id` re-run gets `E_ID_TAKEN` naming its
  own file before `E_PUSH_PENDING` can fire. Re-use after `CreatedUnwritten`
  is unpinned. State the intended behaviour and pin both cases.
- 🔵 **Usability**: `E_ID_WITH_PROJECT` suggests fixes that lead to a second
  refusal
  **Location**: Phase 2 §3 (message text)
  Under a keyless pattern or `{tracker}`, both suggested remedies hit another
  refusal. Tailor the remedy to the pattern.
- 🔵 **Usability**: The shape refusals cite "the pattern's width" without
  stating it
  **Location**: Phase 2 §3 (message text)
  Give the digit width and one example rendered at number 1 with `work.key`,
  and name the pattern in `E_ID_DRAFT_RESERVED`.
- 🔵 **Usability + Standards**: The `--id` help leads with refusals and
  departs from the sibling flags' style
  **Location**: Phase 3 §2
  Lead with the accepted form and an example. Note that later allocation
  continues above the ID. End with a full stop, backtick `--project`, and
  point to `exit_codes` rather than restating it. Update the `create`
  description ("self-allocating its own ID").
- 🔵 **Usability**: A collision surfaces only after drafting, and the
  recovery's resume point is ambiguous
  **Location**: Phase 5 §2 items 4–5
  Say that the new ID is reclassified and the skill resumes at Step 5 with
  the approved draft, and add a Cancel option.
- 🔵 **Usability**: After an accidental high `--id`, the overflow message
  doesn't point to the cause
  **Location**: Decisions "`E_PATTERN_OVERFLOW` wording"
  See the tradeoff above.
- 🔵 **Compatibility**: Case-insensitive allocation is filed under Changed;
  the project files output changes as Breaking
  **Location**: Phase 1 §4
  The `work resolve` case-folding precedent sits under `### Breaking`. Name
  `work create-batch` as well.
- 🔵 **Compatibility**: Free-text topics that mention `--id` will be parsed
  as a requested ID
  **Location**: Phase 5 §2 step 3
  Recognise `--id` only as a leading or trailing flag, and add an eval with
  `--id` in prose.
- 🔵 **Standards**: The `E_ID_` family is an abbreviation and a literal
  prefix of `E_IDENTITY_`
  **Location**: Error Identifiers
  The skill branches on "an `E_ID_` code", and `E_IDENTITY_RUN_BUSY` also
  exits 1 there. Either rename the family to follow the domain type, or have
  the skill list the five codes.
- 🔵 **Standards + Test Coverage**: The distinct-code tests cannot fail on
  distinctness from existing codes
  **Location**: Phase 2 §2 (Refusals); Phase 3 `keyword_exit_codes.rs`
  Pair each variant with its literal. Assert that 14 literals plus five
  `code()` values give 19 distinct codes. Add an exhaustive
  `const fn covered`, and freeze `(code, 1)` pairs.
- 🔵 **Standards**: The `public-api:update && public-api:check` criterion is
  a tautology
  **Location**: Phase 2 Success Criteria
  Keep the update-and-read-the-diff step under Changes, and make the
  criterion `public-api:check` alone.
- 🔵 **Standards**: Multi-filter `cargo test` invocations are invalid
  **Location**: Phase 2 and 3 Success Criteria
  `cargo test` accepts one test-name filter. Use `cargo nextest run -p …`
  with several filters.
- 🔵 **Standards + Test Coverage**: The scope pin misses continuation lines
  and lacks synthetic cases
  **Location**: Phase 5 §1 (`test_explicit_id_scope.py`)
  Join backslash continuations and match `--id` followed by whitespace, `=`
  or end of line. Add `violations(root)` with flagged and compliant
  synthetic trees.
- 🔵 **Test Coverage**: Nothing tests that the taken check fails closed on
  an unlistable corpus
  **Location**: Phase 3 §4
  `FilesystemLister` returns empty silently. Make `drafts/` mode 000 and
  assert `Failed`, with nothing written and nothing sent.
- 🔵 **Test Coverage**: Config-error ordering and dry-run parity for scheme
  errors are unpinned
  **Location**: Phase 4 §1; Error Identifiers
  Under `{key}-{number:04d}` with no `work.key`, `--id XY-0400 --project XY`
  should give `E_WORK_KEY_REQUIRED` on both paths.
- 🔵 **Test Coverage**: No eval covers an `E_ID_` refusal from the pushed
  create at step 7.3
  **Location**: Phase 5 §1
  The dry-run's collision check is advisory, so 7.3 can refuse after a clean
  dry-run. Add an eval for it.

#### Suggestions

- 🔵 **Code Quality + Standards**: Variant names don't mirror their codes
  (`AlongsideProject`, `TrackerOwnsIds`). **Location**: Phase 2 §3
- 🔵 **Code Quality**: Exporting `next_number::is_md_filename` leaves the
  duplicate in `resolve.rs`, which was verified at lines 210 and 54. Move it
  to one shared module. **Location**: Phase 2 §1
- 🔵 **Standards**: The name `HeldIds.files` collides with
  `WorkItemFiles::files()`, which includes `drafts/`. **Location**: Phase 2
  §3
- 🔵 **Architecture**: `CreationContext::requested` is an `Option` that the
  tracker strategies ignore. Consider
  `LocalIdSource { Allocated, Requested }`. **Location**: Phase 3 §4
- 🔵 **Correctness**: The greedy key run refuses valid IDs under
  `{key}v{number:03d}`. Document or refuse such patterns. **Location**:
  Phase 2 §3
- 🔵 **Correctness + Test Coverage**: Add boundary cases: a requested number
  that overflows `u64` under `{number:020d}`, an oversized filename digit
  run, and an empty `--project ''`. **Location**: Decisions "Number range";
  Phase 2 §2
- 🔵 **Correctness**: `case_insensitive(true)` folds Unicode case, while the
  held-ID rule ignores ASCII case only. Build with `.unicode(false)`.
  **Location**: Phase 1 §2
- 🔵 **Usability + Compatibility**: The `--dry-run` help should say a pass
  doesn't reserve the ID. Ship Phases 3 and 4 together, or have Phase 3 use
  `conflicts_with`. **Location**: Phase 4 §2; Implementation Approach
- 🔵 **Usability**: Mention `--id` in the no-argument prompt and the Work
  Items guide, and accept `--id=<ID>`. **Location**: Phase 5 §2, §4
- 🔵 **Compatibility**: Exit 1 contradicts the documented band ("an internal
  error"); say in the paragraph that the choice is deliberate.
  **Location**: Phase 3 §5
- 🔵 **Compatibility**: Add admitted and taken cases under the deprecated
  `{project}-{number:04d}`. **Location**: Phase 2 §2
- 🔵 **Test Coverage**: Keep the full matrices in the domain, and only the
  wiring-sensitive cases in `create.rs`. **Location**: Phase 3 §1

### Strengths

- ✅ The collision check takes the place of `allocate_local_id` inside
  `create_local_item`'s closure, under the create lock and before
  `refuse_to_overwrite` and `push_legacy_item`. So the lock race and "no
  tracker request on refusal" hold by construction.
- ✅ Checks 1–4 run after `resolve_scheme` and before `create_dir_all`. As a
  result, `E_ID_WITH_PROJECT` precedes `E_PATTERN_KEY_UNUSED`, and a refusal
  leaves no empty `meta/work/`.
- ✅ The domain model sits in the regex-free `work` crate, as ADR-0053 and
  the pup import rule require. Only `admit` can build a `RequestedId`, so
  mismatch-before-collision is enforced by types.
- ✅ Refusals render once through `Display`, following `NotRekeyed`, so the
  real create, the dry-run and 0371 share one text.
- ✅ The collision check takes no lock guard, built for 0371's lock-free
  validation. A plain dry-run stays byte-identical.
- ✅ `compile_ignoring_case` is a new constructor, so the visualiser is
  untouched. One `key_sequence::scanner_for` replaces two duplicated scanner
  builds, and keeps `work next-number`'s projection in step with the write.
- ✅ Every acceptance criterion traces to a named test. Several tests are
  aimed at specific mutations: the lock race with a different title,
  `a_different_key_is_not_taken`, and the `0000`/`0001`/`9999`/`10000`
  brackets.
- ✅ "No request of any kind" is asserted through `tracker.calls()`, which
  records `PreviewCreate` too. Parity tests compare against the no-`--id`
  run rather than hard-coding outcomes.
- ✅ The skill's corrections are limited to padding and adding the key,
  each confirmed by the user. The skill never chooses for the user, and the
  push gate's `E_ID_*` branch keeps a taken ID out of "save locally".
- ✅ The skill and the binary ship in lockstep, so the skill can never pass
  `--id` to a binary without it.

### Recommended Changes

1. **Record the ADR-0045 departure, or move classification into the CLI**
   (addresses: deterministic ID classification in skill prose)
   Add a Decisions row that names ADR-0045, the third copy of the grammar,
   and the cost: a valid ID the skill wrongly rejects is never backstopped.
   Either have the skill compare its proposal with `work canonicalise-id`,
   or name a follow-up for 0371's validate-only dry-run.
2. **Consolidate the pattern grammar and the held-ID rule in `work`**
   (addresses: grammar walked three times; equivalence rule private;
   refactor-first changes behaviour; malformed patterns unspecified)
   Move the walking primitives and a named lenient held-ID parse into
   `id_pattern.rs`. Derive `pattern_cap` from `segments`. Keep the Phase 2
   refactor a pure move, with new `resolve` pins under `{number}` and
   `{key}-{number:06d}`. Validate the pattern through `compile_scan_regex`
   before admission. State how the rule relates to `rekey::sequence`.
3. **Let the domain choose the filename scanner's key, and make the fixtures
   prove the wiring** (addresses: unenforced scanner convention; filename
   wiring masked)
   Replace `scanner_for_requested_key` with a key-taking factory or port.
   Have the Phase 3 filename holders carry no matching frontmatter `id`.
4. **Enrich and tailor the refusal messages** (addresses: `E_ID_TAKEN`
   field and value; `E_ID_WITH_PROJECT` remedies; width not stated; variant
   names)
   Add `held_as` and `held` to `Taken`, and render a repository-relative
   path. Tailor the `--project` remedy to the pattern. State the digit width
   with an example. Align the variant names with the codes, and decide on
   the `E_ID_` family name.
5. **Decide how other ID readers treat `--id` shapes** (addresses: existing
   readers mishandle shapes; visualiser asserted, not examined)
   Recommended: add a What We're NOT Doing entry naming `normalise_id`,
   `is_canonical_id_token` and the visualiser's scan. Raise a follow-up work
   item, and add pin tests for `xy-0400` and `X1-0400`.
6. **Make the CLI tests able to fail** (addresses: `meta_tree` blind spot;
   distinct-code tests; unlistable corpus; config-error ordering;
   pending-push retries)
   Assert that `meta/work` doesn't exist. Build the code sets from
   production `code()` values with an exhaustive `covered`. Add the
   fail-closed, config-error and pending-push tests.
7. **Tighten the skill and its gate** (addresses: eval gate; Bash quoting;
   correction to `0000`; non-`E_ID_` exit 1; `--id` in a topic; 7.3 eval;
   recovery resume point; scope-pin coverage)
   Require a 1.0 pass rate for evals 39 to 50 and record the red runs as
   `old_skill`. Single-quote `--id`. Require corrections to land on Exact.
   Add a branch for a non-`E_ID_` exit 1. Recognise `--id` only as a leading
   or trailing flag. Add evals for step 7.3, a zero request, and `--id` in
   prose. Spell out the resume point. Restructure the scope pin like its
   sibling guards.
8. **Share one dry-run preflight and narrow the parity claim** (addresses:
   dry-run duplication and order; advisory help)
   Extract a helper that `creation_context` and the dry-run both call. Limit
   the parity wording to the `--id` refusals. Say in `--dry-run` help that a
   pass reserves nothing. Land Phases 3 and 4 together, or use
   `conflicts_with` in between.
9. **Specify path- and panic-safety in the exact walk** (addresses: path
   safety invariant; character-boundary comparison; Unicode folding; `u64`
   overflow)
   Use `is_ascii_*` predicates, `str::get`-based comparisons and a checked
   number parse. Build the scanner with `.unicode(false)`. Add the hostile
   and overflow inputs to the mismatch matrix.
10. **Housekeeping** (addresses: changelog section; exit-band paragraph;
    help style; `cargo test` filters; `public-api` criterion;
    `is_md_filename`; `HeldIds.files`; `{project}` pattern)
    Move Phase 1's changelog entry to `### Breaking` and name
    `create-batch`. Explain exit 1 in `exit_codes.rs`. Fix the help style.
    Switch to `cargo nextest run`. Make the criterion `public-api:check`.
    Dedupe `is_md_filename`. Rename `HeldIds.files`. Add `{project}` cases.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound. The domain model lives in the
regex-free `work` core, the collision check sits at the existing lock-held
`allocate` seam before the push, and the lock-free check signature is built
to fit 0371. The main risks are in domain coherence and the division of
labour:

- the skill takes on deterministic classification, which ADR-0045 assigns
  to the CLI;
- the equivalence rule and the width-aware walker sit beside existing
  implementations that give different answers;
- the taken rule depends on an unenforced convention for the injected
  scanner.

**Strengths**: reuses the `allocate` seam under the lock before the push;
the domain sits in `work`, within ADR-0053 and pup; `refuse_if_taken` is
pure over data already read; the lock-free check signature fits 0371;
admission precedes `create_dir_all`; `compile_ignoring_case` leaves the
visualiser untouched; the dry-run without `--id` is byte-identical; scope
deferrals are explicit.

**Findings**:

- 🟡 major / high — **Deterministic ID classification sits in skill prose
  against ADR-0045, unacknowledged** (Phase 5 §2 step 3; Decisions). Skill
  and CLI can disagree on what is exact, and the backstop misses wrongful
  rejections. Record the departure, and move classification towards the
  CLI or name a follow-up for 0371's validate-only dry-run.
- 🟡 major / medium — **The key-and-number equivalence rule is buried in
  `RequestedId`, while `identity.rs` and `rekey::sequence` keep their own**
  (Phase 2 §3). There are three rules that can disagree on `400` against
  `0400` or `pp-0500` against `PP-0500`. Name the rule as a domain concept
  and state how it relates to `rekey::sequence`.
- 🔵 minor / medium — **Another width parser and grammar walker, outside the
  parity test; a malformed pattern surfaces as `E_ID_PATTERN_MISMATCH`**
  (Phase 2 §1). Derive `pattern_cap` from `segments`, validate through
  `compile_scan_regex` before admission, and keep resolve's acceptance of
  the number token, or state the change.
- 🔵 minor / medium — **The taken rule depends on an unenforced convention
  for the injected scanner** (Phase 2 §3; Phase 1 §3). Define a
  `KeySequenceScanners` port in `work`, implemented once in an adapter.
- 🔵 minor / medium — **The dry-run preflight repeats `creation_context`'s
  opening and orders the checks differently** (Phase 4 §2). Extract one
  helper, and state that parity covers only the `--id` refusals until 0371.
- 🔵 minor / medium — **A pushed `--id` create retried after an unknown
  outcome is unaddressed** (Phase 3 §4). `E_ID_TAKEN` fires before
  `E_PUSH_PENDING`. State the intended behaviour and pin it with a
  `RecordingTracker` test.
- 🔵 suggestion / low — **The requested ID is an `Option` on the shared
  context, which the tracker strategies ignore** (Phase 3 §4). Model it as
  `LocalIdSource { Allocated, Requested(RequestedId) }`.
- 🔵 suggestion / low — **The visualiser's handling of `--id` items is
  asserted, not examined** (What We're NOT Doing). Add the expected
  behaviour to Current State Analysis, or a pin test in the indexer.

### Code Quality

**Summary**: The plan is broadly maintainable. It has one refusal enum with
`code()` and `Display`, one `id_for` seam, and one shared scanner builder.
The main risks are pattern-grammar logic copied between `resolve.rs` and
`requested_id.rs`, a dry-run path that repeats the check order, and a
`Taken` refusal that explains less than the existing retirement refusal.

**Strengths**: the refusal enum follows `NotRekeyed`; names follow the
domain; `admit` is a guard sequence; `id_for` reuses the closure seam;
`key_sequence::scanner_for` removes a duplication; `compile_ignoring_case`
is named explicitly; `segments` is shared, not rewritten; the public-API
diff is reviewed; the change is opt-in and additive.

**Findings**:

- 🟡 major / medium — **The pattern-walking grammar is re-implemented three
  times, and the width parsed twice, in `work`** (Phase 2 §1, §3). Grammar
  changes must land in three walkers and two width parsers, and drift
  disagrees silently. Move the walking primitives into `id_pattern.rs`,
  derive `pattern_cap` from `Segment::Number { width }`, and consider a
  named lenient result type.
- 🔵 minor / high — **The dry-run duplicates `creation_context`'s resolution
  and check order** (Phase 4 §2). Extract an `admitted_request` helper, and
  match `args.id.as_deref()` once.
- 🔵 minor / medium — **The `Taken` refusal names the path but not what
  holds the ID** (Phase 2 §3). Add `held_as` and `held`, and render a
  repository-relative path, as `RetirementRefusal` and `NotRekeyed` do.
- 🔵 minor / medium — **The "refactor first" step changes behaviour and adds
  an unread field** (Phase 2 §1). Keep it a pure move, and add `width` in
  the green step, driven by tests.
- 🔵 suggestion / medium — **`HeldIds` needs a caller-built, key-specific
  scanner and two directory reads** (Phase 2 §3; Phase 3 §4). Parse filename
  prefixes with the same lenient walk, or inject a key-taking factory.
- 🔵 suggestion / high — **Exporting `next_number::is_md_filename` leaves
  the duplicate in `resolve.rs`** (Phase 2 §1). Move it to one shared
  module.
- 🔵 suggestion / medium — **Refusal variant names don't match their `E_`
  codes** (Phase 2 §3). Use `WithProject` and `TrackerOwned`, or rename the
  codes.

### Test Coverage

**Summary**: The testing strategy is thorough and layered, traces every
criterion to a named test, and aims several tests at specific mutations.
The main gaps are assertions that cannot fail on what they claim to pin:

- `meta_tree` cannot see an empty directory;
- frontmatter matches may mask the filename-scanner wiring;
- the frozen-code table is close to a tautology;
- the scope pin misses continuation lines;
- the eval gate lets new evals fail.

Some error paths are also unpinned.

**Strengths**: a full traceability table; mutation-aimed tests (lock race
with a different title, different key, range brackets);
`tracker.calls().is_empty()` covers previews; parity tests compare against
the no-`--id` run; characterisation pins are marked as passing from the
start; types enforce admission order; evals are run red before the skill
is edited.

**Findings**:

- 🟡 major / high — **`meta_tree` cannot detect an empty `meta/work/`**
  (Phase 3 §1; Phase 4 §1). Assert `!repo.path("meta/work").exists()` in the
  admission matrix and in every dry-run case.
- 🟡 major / medium — **The filename-scan wiring may be masked by
  frontmatter matches** (Phase 3 §1; Phase 2 Taken rule). Make the filename
  holders carry no matching `id`, at least for `xy-0400-other.md` and
  `00400-x.md`.
- 🟡 major / high — **The eval gate lets new evals fail outright, and the
  red runs are discarded** (Phase 5 §1, §3). Require 1.0 for evals 39 to 50,
  and record the red runs as `old_skill`.
- 🔵 minor / high — **The line-based scope pin misses `--id` on continuation
  lines** (Phase 5 §1). Join continuations, tighten the `--id` match, and
  add a synthetic positive case.
- 🔵 minor / high — **The frozen-code test cannot fail on distinctness from
  existing identifiers** (Phase 3 `keyword_exit_codes.rs`; Phase 2
  Refusals). Build the set from production `code()` values and add an
  exhaustive `covered`.
- 🔵 minor / medium — **Nothing tests that the taken check fails closed on
  an unlistable corpus** (Phase 3 §4). Model the test on
  `a_created_marker_over_an_unlistable_corpus_fails_rather_than_reusing_its_key`.
- 🔵 minor / medium — **Pending-push re-runs under `--id` are untested**
  (Phase 3 Push). Pin the `CreatedUnwritten` re-use and the `loud-terminal`
  re-run.
- 🔵 minor / medium — **Config-error ordering and dry-run parity for scheme
  errors are unpinned** (Phase 4 §1; Error Identifiers). Assert
  `E_WORK_KEY_REQUIRED` before `E_ID_WITH_PROJECT` on both paths.
- 🔵 minor / medium — **No eval covers an `E_ID_` refusal from the pushed
  create (7.3)** (Phase 5). Add a clean dry-run followed by a refused push
  create.
- 🔵 minor / medium — **Existing `resolve` tests don't cover the branch the
  `segments` refactor changes** (Phase 2 §1). Pin full-ID resolution under
  `{number}` and `{key}-{number:06d}` first.
- 🔵 suggestion / medium — **The composition layer duplicates the full
  domain matrices** (Phase 3 §1). Keep one per class plus the
  wiring-sensitive cases.
- 🔵 suggestion / low — **A few branch boundaries lack a case** (Phase 2
  §2): an empty `--project ''`, an oversized filename digit run, and an
  over-wide requested number.

### Correctness

**Summary**: The core logic is sound. Check-then-write is atomic under the
lock and before any tracker request, admission comes before any side
effect, and the refusal order matches the work item. The weak spots are at
the edges: dry-run parity holds only when the other checks pass, malformed
and unusual patterns are unspecified, the lenient parse risks a panic, and
the skill's decision table has two gaps.

**Strengths**: the collision check replaces `allocate_local_id` in the
closure; admission placement gives `E_ID_WITH_PROJECT` before
`E_PATTERN_KEY_UNUSED`; bare `{number}` as width 4 agrees with the cap;
the held-ID `u64` overflow falls back to whole-string comparison; the
filename scan is keyed on the requested key; `compile_ignoring_case` is
opt-in.

**Findings**:

- 🔵 minor / medium — **The dry-run collision refusal can differ from the
  real create's** (Phase 4). Narrow the claim to apply when the non-`--id`
  checks pass, and record the gap as 0371's.
- 🔵 minor / medium — **Unrecognised number specs leave admission
  unspecified and change `work resolve`** (Phase 2 §1). Validate the pattern
  before admission, state that an empty or number-less segment list admits
  nothing, and keep or test the `resolve` change.
- 🔵 minor / low — **The case-insensitive literal comparison must respect
  character boundaries** (Phase 2 §3). Use
  `rest.get(..n).is_some_and(..)`, and add a non-ASCII held-ID case.
- 🔵 minor / medium — **A skill correction can produce an ID the CLI will
  refuse** (Phase 5 §2 Step 0). Require the result to meet Exact, and add
  `"0"` to eval 46.
- 🔵 minor / medium — **Non-`E_ID_` exit-1 refusals from the `--id` dry-run
  are misread as tracker failures** (Phase 5 §2 Step 5, 7.1). Relay and
  abort.
- 🔵 suggestion / low — **The greedy key run cannot admit IDs when the
  literal after `{key}` starts with an alphanumeric** (Phase 2 §3). Stop at
  the shortest match, or document and refuse such patterns.
- 🔵 suggestion / low — **A requested number can overflow `u64` under
  patterns wider than 19 digits** (Decisions "Number range"). Treat it as a
  mismatch.
- 🔵 suggestion / low — **The case-insensitive regex folds Unicode case
  while the held-ID rule is ASCII-only** (Phase 1 §2). Use `.unicode(false)`
  or `(?i-u)`.

### Usability

**Summary**: The `--id` contract is clear. It has five ordered refusals with
no side effects, dry-run parity, early classification in the skill, and a
skill that never chooses for the user. The main problems are in the
messages: `E_ID_TAKEN` hides why a holder takes the ID, `E_ID_WITH_PROJECT`
suggests fixes that refuse again, and the shape errors don't state the
width. Help text, discoverability and the recovery resume point are lesser
issues.

**Strengths**: distinct codes rendered once; admission before
`create_dir_all`; the dry-run refuses identically without waiting on the
lock; `{tracker}` and no-exact-form are caught before composing;
corrections are minimal and confirmed; the push-gate `E_ID_*` branch
precedes "save locally"; the projection agrees with the write.

**Findings**:

- 🟡 major / high — **`E_ID_TAKEN` names the holder's path but not the
  field or value** (Phase 2 §3). Carry the source and the held value, and
  render a repository-relative path.
- 🔵 minor / high — **`E_ID_WITH_PROJECT` suggests fixes that lead to a
  second refusal** (Phase 2 §3). Tailor the remedy to the pattern.
- 🔵 minor / high — **The shape refusals cite "the pattern's width" and
  "shape" without stating either** (Phase 2 §3). State the width, give a
  rendered example, and name the pattern in `E_ID_DRAFT_RESERVED`.
- 🔵 minor / medium — **The `--id` help lists refusals but not the expected
  form, and the `create` description goes stale** (Phase 3 §2). Lead with
  the positive form, and note that allocation continues above the ID.
- 🔵 minor / medium — **A collision surfaces after the whole drafting
  conversation, and the recovery's resume point is ambiguous** (Phase 5 §2
  items 4–5). Resume at Step 5 with the approved draft, and add a Cancel
  option.
- 🔵 minor / medium — **After an accidental high `--id`, the overflow
  message doesn't point to the cause** (Decisions; Required Work Item
  Amendment). Consider naming `highest_file` in the at-cap branch.
- 🔵 suggestion / medium — **The `--dry-run` help should say the `--id`
  check is advisory** (Phase 4 §2). Consider landing Phases 3 and 4
  together.
- 🔵 suggestion / medium — **The skill's `--id` is hard to discover beyond
  the argument hint** (Phase 5 §2, §4). Mention it in the welcome text and
  the Work Items guide, and accept `--id=<ID>`.
- 🔵 suggestion / low — **`HeldIds` leaves building the key-specific scanner
  to the caller** (Phase 2 §3). Let the domain choose the key.

### Compatibility

**Summary**: The CLI change is additive. It leaves existing invocations
byte-identical, freezes the error codes, regenerates the pinned API with a
diff review, and ships the skill and binary in lockstep. The main gap is
downstream: `--id` writes IDs under any key, in any case and with digits in
the key, but only the allocators were adjusted. Phase 1 is also filed under
Changed rather than Breaking.

**Strengths**: no behaviour change without `--id`; `FROZEN_CREATE_CODES`
covers the 14 existing codes; `Segment` stays `pub(crate)`; the visualiser's
case-sensitive scan is pinned; one shared scanner keeps the projection and
the write in step; case-insensitive comparison behaves the same on APFS and
Linux; the design is forward-compatible with 0371; the skill and binary
ship in lockstep.

**Findings**:

- 🟡 major / high — **Existing ID readers reject or mishandle the shapes
  `--id` now writes** (Decisions; What We're NOT Doing). This affects
  `normalise_id`, the visualiser's slug scan and `is_canonical_id_token`
  (`work sync` dossiers). Align them, or record a non-goal with a follow-up
  and pin tests.
- 🔵 minor / medium — **Case-insensitive allocation is filed under Changed;
  the project files output changes as Breaking** (Phase 1 §4). Move it, and
  name `create-batch`.
- 🔵 minor / medium — **Free-text topics that mention `--id` will be parsed
  as a requested ID** (Phase 5 §2 Step 0). Accept the flag only in a
  leading or trailing position, and add an eval.
- 🔵 suggestion / low — **A release between Phases 3 and 4 would ship a
  looser `--dry-run --id` contract** (Implementation Approach). Ship them
  together, or use `conflicts_with`.
- 🔵 suggestion / low — **Exit 1 for `E_ID_TAKEN` conflicts with the
  documented exit-code bands** (Phase 3 §5). Explain the choice in the
  paragraph.
- 🔵 suggestion / medium — **No test covers the deprecated `{project}`
  token under `--id`** (Phase 2 §2). Add an admitted case and a taken case.

### Security

**Summary**: As a local CLI plus agent skill, the plan is sound. The `--id`
is checked against an exact-shape allowlist and becomes a `RequestedId`
that only `admit` can build. The collision check runs under the lock in
the same step as the write, and every refusal precedes any tracker
request. Two gaps remain: path safety rests on an unstated invariant,
partly enforced as a side effect of a later check, and the skill puts the
ID into Bash with no quoting rule.

**Strengths**: `RequestedId` is only constructible via `admit`; the
allowlist refuses rather than normalises; the collision check is atomic
under the lock; nothing is sent before a refusal; draft and tracker IDs
cannot be forged; the scan regex is guarded twice and runs in linear time;
the skill confirms every correction with the user.

**Findings**:

- 🔵 minor / medium — **Path safety of `--id` rests on an unstated, partly
  incidental invariant** (Phase 2 §3; §2). Specify ASCII predicates and a
  non-panicking parse, add traversal and non-ASCII inputs to the matrix, and
  make pattern-literal validation part of admission.
- 🔵 minor / medium — **The skill puts the requested ID into Bash unquoted;
  the CLI cannot backstop shell parsing** (Phase 5 §2 Step 5; Decisions).
  Single-quote the ID, put only Exact or confirmed IDs into commands, refuse
  a `'`, and add an eval expectation.

### Standards

**Summary**: The plan fits the project's conventions: the domain type sits
within the pup rule, refusals render through `Display`, and the skill reuses
existing injection lines and argument-hint shapes. The gaps are consistency
issues:

- the code family and variant names drift from the `NotRekeyed` precedent;
- the distinct-code tests lack the exhaustive guard;
- the help style differs from the sibling flags;
- some verification commands don't match the project's runner or the
  `public-api` guidance.

**Strengths**: the message contract and the pattern quoting match the
existing `E_PATTERN_*` messages; the module placement and the `public-api`
diff review follow tasks/README; the skill edits follow existing
conventions; the test helpers and names mirror existing fixtures; the
changelog and `exit_codes.rs` homes are correct.

**Findings**:

- 🔵 minor / medium — **The `E_ID_` family is an abbreviation and a literal
  prefix of `E_IDENTITY_`** (Error Identifiers). Mirror the domain type, or
  have the skill name the five codes.
- 🔵 suggestion / medium — **Refusal variant names don't mirror their code
  suffixes** (Phase 2 §3). Use one vocabulary for each pair.
- 🔵 minor / high — **The distinct-code tests omit the established
  exhaustive-coverage guard** (Phase 2 §2; Phase 3). Add `const fn covered`,
  and freeze `(code, 1)` pairs.
- 🔵 minor / high — **The `--id` help text departs from the sibling flags'
  style** (Phase 3 §2). Add a trailing full stop, backtick `--project`, and
  point to `exit_codes`.
- 🔵 minor / medium — **The `public-api:update && public-api:check`
  criterion is a tautology** (Phase 2 Success Criteria). Make it
  `public-api:check`.
- 🔵 minor / medium — **Multi-filter `cargo test` invocations don't match
  cargo's CLI or the project's runner** (Phase 2 and 3 Success Criteria).
  Use `cargo nextest run`.
- 🔵 minor / medium — **The new tree guard lacks the synthetic cases its
  sibling guards carry** (Phase 5 §1). Add `violations(root)` with flagged
  and compliant trees.
- 🔵 suggestion / low — **`HeldIds.files` reuses a name that means
  something else nearby** (Phase 2 §3). Rename it to `listed_filenames` or
  `top_level_paths`.

## Re-Review (Pass 2) — 2026-10-30

**Verdict:** REVISE

The revision resolved 49 of the 60 first-pass findings and partly resolved
the other 11; none is still present. The `--id` path itself is now sound. The
new risks sit in the relocation phases (1 to 5), where removing the regex
compiler and consolidating the canonicaliser change behaviour the plan does
not yet pin. The phase numbers below are the revised plan's.

### Previously Identified Issues

Architecture:

- 🟡 **Architecture**: ID classification in skill prose against ADR-0045 — Resolved
- 🟡 **Architecture**: Key-and-number rule private to `RequestedId` — Partially resolved (`identity::holder_of` still compares whole strings)
- 🔵 **Architecture**: Second width parser; malformed pattern as a mismatch — Resolved
- 🔵 **Architecture**: Unenforced convention for the injected scanner — Resolved
- 🔵 **Architecture**: Dry-run preflight duplicated and reordered — Resolved
- 🔵 **Architecture**: Pushed `--id` retry after an unknown outcome — Resolved
- 🔵 **Architecture**: `Option` on the context that tracker arms ignore — Resolved
- 🔵 **Architecture**: Visualiser handling asserted, not examined — Partially resolved (key-case joins unexamined)

Code Quality:

- 🟡 **Code Quality**: Grammar walked three times, width parsed twice — Partially resolved (exact walk in two places; `canonical_digit_width` left)
- 🔵 **Code Quality**: Dry-run repeats `creation_context` — Resolved
- 🔵 **Code Quality**: `Taken` names the path, not the holding — Resolved
- 🔵 **Code Quality**: Refactor-first step changes behaviour — Resolved
- 🔵 **Code Quality**: `HeldIds` caller-built scanner — Resolved
- 🔵 **Code Quality**: `is_md_filename` duplicate — Resolved
- 🔵 **Code Quality**: Variant names don't match codes — Resolved

Test Coverage:

- 🟡 **Test Coverage**: `meta_tree` blind to an empty `meta/work/` — Resolved
- 🟡 **Test Coverage**: Filename wiring masked by frontmatter — Resolved
- 🟡 **Test Coverage**: Eval gate lets new evals fail — Resolved
- 🔵 **Test Coverage**: Scope pin misses continuation lines — Resolved
- 🔵 **Test Coverage**: Frozen-code test cannot fail — Resolved
- 🔵 **Test Coverage**: Unlistable corpus untested — Resolved
- 🔵 **Test Coverage**: Pending-push re-runs untested — Resolved
- 🔵 **Test Coverage**: Config-error ordering unpinned — Resolved
- 🔵 **Test Coverage**: No eval for a 7.3 refusal — Resolved
- 🔵 **Test Coverage**: `resolve` tests miss the refactored branch — Partially resolved (two of three pins cannot fail)
- 🔵 **Test Coverage**: Composition duplicates domain matrices — Resolved
- 🔵 **Test Coverage**: Branch boundaries without cases — Resolved

Correctness:

- 🔵 **Correctness**: Dry-run refusal can differ from the real create's — Partially resolved (pending-push re-run diverges)
- 🔵 **Correctness**: Unrecognised number specs — Resolved
- 🔵 **Correctness**: Character-boundary safety — Resolved
- 🔵 **Correctness**: Skill correction to `0000` — Resolved
- 🔵 **Correctness**: Non-`E_ID_` exit 1 misread — Resolved
- 🔵 **Correctness**: Greedy key run — Partially resolved (now drives allocation; see new major)
- 🔵 **Correctness**: Requested number overflows `u64` — Resolved
- 🔵 **Correctness**: Unicode case folding — Resolved

Usability:

- 🟡 **Usability**: `E_ID_TAKEN` omits the holding — Resolved
- 🔵 **Usability**: `E_ID_WITH_PROJECT` remedies refuse again — Partially resolved (keyed text, bare-number `--id`)
- 🔵 **Usability**: Shape refusals omit width and shape — Resolved
- 🔵 **Usability**: `--id` help; stale `create` description — Resolved
- 🔵 **Usability**: Late collision; ambiguous resume — Partially resolved (6.3 resume point; evals 51 and 54)
- 🔵 **Usability**: Overflow message misses the cause — Resolved
- 🔵 **Usability**: `--dry-run` help advisory — Resolved
- 🔵 **Usability**: `--id` hard to discover — Resolved
- 🔵 **Usability**: Caller builds the key-specific scanner — Resolved

Compatibility:

- 🟡 **Compatibility**: Readers mishandle the shapes `--id` writes — Partially resolved (canonical IDs keep key case)
- 🔵 **Compatibility**: Case-insensitive allocation under Changed — Resolved
- 🔵 **Compatibility**: Topics mentioning `--id` — Resolved
- 🔵 **Compatibility**: Interim `--dry-run --id` contract — Resolved
- 🔵 **Compatibility**: Exit 1 against the bands — Resolved
- 🔵 **Compatibility**: `{project}` untested — Resolved

Security:

- 🔵 **Security**: Path safety rests on an unstated invariant — Resolved
- 🔵 **Security**: ID unquoted in Bash — Partially resolved (the corrected ID is not quote-checked)

Standards:

- 🔵 **Standards**: `E_ID_` family prefix — Resolved
- 🔵 **Standards**: Variant names — Resolved
- 🔵 **Standards**: Exhaustive guard missing — Resolved
- 🔵 **Standards**: `--id` help style — Resolved
- 🔵 **Standards**: `public-api` tautology — Resolved
- 🔵 **Standards**: Multi-filter `cargo test` — Partially resolved (Phase 5 package names; visualiser lane)
- 🔵 **Standards**: Guard lacks synthetic cases — Resolved
- 🔵 **Standards**: `HeldIds.files` name — Resolved

### New Issues Introduced

Major:

- 🟡 **Security + Code Quality + Test Coverage + Compatibility + Correctness + Architecture**: Grammar validation is no longer guaranteed once the regex compiler goes — `compile_scan_regex` is today the only rule-2/rule-5 check for allocation, `--project`, `work next-number`, rekey and visualiser compose. Phases 3 to 5 remove it, `resolve_scheme` does not validate, and no test pins a refusal; `work create --project '../x'` could write outside the work directory, and a malformed pattern makes the visualiser silently empty.
- 🟡 **Code Quality + Architecture + Compatibility + Test Coverage + Standards**: Phase 5 deletes `corpus::IdScanner` that `work` still uses — `PatternScanner`, `allocate`'s signature and the pup comment name it; the trait's home is unstated, so Phase 5 cannot compile, and the `work` public-API fixture is missing from its list.
- 🟡 **Code Quality + Architecture**: No crate-legal seam carries a requested ID into `create_local_item` — `LocalIdSource` lives in `work-cli`, which `work-adapters` cannot import, yet the pending-push precondition must run inside `create_local_item` before the taken check.
- 🟡 **Code Quality + Compatibility**: Moving `canonicalise-id` onto `resolve_scheme` changes its refusals — `E_WORK_KEY_REQUIRED` and the `{tracker}` integration check replace `E_PATTERN_MISSING_KEY`, breaking a pinned golden, against "unchanged in behaviour".
- 🟡 **Test Coverage**: Moved scheme tests assert the semantics Phase 4 changes — `OTHER-0040` not canonical, `ENG-0042` passing through under `{number:04d}`, width 0 for `{number}`; the behaviour changes would be recorded after the fact, not driven red.
- 🟡 **Compatibility + Correctness**: Widening `is_canonical_id_token` to any key changes slugs — through `is_identifier`, dated documents such as `2026-10-29-iso-8601-dates.md` lose leading tokens under keyed patterns, changing visualiser slugs and clusters unannounced.
- 🟡 **Correctness + Usability**: The non-backtracking key run breaks allocation under `{key}v{number:03d}` — the same walk now drives allocation, scans and canonicalisation, so every create allocates `PPv001`, writing duplicate IDs; the mismatch refusal's example is the refused input.

Minor:

- 🔵 **Correctness + Compatibility + Architecture**: Visualiser joins build keys by three rules — own `id` verbatim, references re-padded, paths in the configured case; `id: PP-42` stops clustering with `work-item:PP-42`, and `pp-0500` joins neither form.
- 🔵 **Correctness**: The one canonicaliser changes cluster keys under `{tracker}` and keyless patterns — free text becomes a cluster key; `ENG-0042` stops joining under `{number:04d}`.
- 🔵 **Correctness + Compatibility + Architecture**: `numeric()` does not reproduce the legacy linkage reads — `0042.md`, `0042_x.md` and `12345` resolve differently, while the linkage tests switch to a double.
- 🔵 **Architecture + Code Quality + Standards + Correctness**: Migrate's identity arrives through three seams, and a `&dyn` return cannot have a default-erroring body — one migrate-owned port with `Result` methods fits the `SyncBaselines` precedent.
- 🔵 **Correctness + Architecture**: The dry-run and `work id check` report `E_ID_TAKEN` on a pending-push re-run where the real create reports `E_PUSH_PENDING`.
- 🔵 **Code Quality + Test Coverage**: The taken check fails open on an unreadable holder file — `markdown_files_in` drops it with its filename.
- 🔵 **Test Coverage + Correctness**: Two parity tests contradict `HeldId`'s semantics — oversized digit runs and raw digit spans (`0042` vs `42`); keyless `parse_full_id` ignores literals.
- 🔵 **Architecture**: Phase 3 depends on Phase 1 (`tracker::is_tracker_key`), though declared independent.
- 🔵 **Architecture**: `identity::holder_of` remains a second "already held" rule.
- 🔵 **Code Quality**: Error types and validators overlap (`MissingKey` twice; two `validate` functions).
- 🔵 **Code Quality**: `ExampleShape` cannot be built for the malformed patterns `admit` refuses.
- 🔵 **Code Quality**: The exact walk is written twice, and `canonical_digit_width` is left.
- 🔵 **Code Quality**: `HeldId` is named for one use; the taken rule is split across modules.
- 🔵 **Code Quality**: `settle_id_scheme` is general setup filed under requested IDs.
- 🔵 **Security**: The validate parity pins one of nine hostile characters and no key value.
- 🔵 **Security**: The corrected ID is not quote-checked; a leading `-` is read as a flag.
- 🔵 **Usability**: Refusals shared with `work id check` tell its user to drop `--id`.
- 🔵 **Usability**: The keyed `E_ID_WITH_PROJECT` remedy refuses a bare-number `--id` again.
- 🔵 **Usability**: `E_ID_PATTERN_MISMATCH` has no remedy, and its clause describes only the number.
- 🔵 **Usability**: "Give that item a lower ID" has no command behind it.
- 🔵 **Usability**: The retired verbs fail with clap's generic error and no pointer.
- 🔵 **Usability**: Recovery resumes at a dry-run the 6.3 path never had; evals 51 and 54 expect two options.
- 🔵 **Usability**: The welcome text offers `--id` where Step 0 cannot parse it.
- 🔵 **Usability**: An abort after drafting does not print the approved draft.
- 🔵 **Test Coverage**: Two of three `resolve` pins resolve through the bare-number branch.
- 🔵 **Test Coverage + Standards**: The retired-verb guard misses wrapped mentions and lacks the guard shape.
- 🔵 **Test Coverage**: The red-first rule cannot be met by characterisation evals (52, possibly 49 and 50).
- 🔵 **Test Coverage**: Skill branches without an eval, notably a re-key refusal from `work id check`.
- 🔵 **Test Coverage**: Pattern-first validation order unpinned without `--id`.
- 🔵 **Test Coverage**: Phase 5 drops the anchor parity comparator; the corpus guard's vocabulary is narrow.
- 🔵 **Standards**: Phase 5 names packages that do not exist (`accelerator-corpus`, `accelerator-migrate`).
- 🔵 **Standards**: Visualiser tests run through nextest, which the project excludes.
- 🔵 **Standards + Code Quality**: The port needs a doc comment its method name should carry; "anchor" collides with corpus's "anchored".
- 🔵 **Standards**: Pedantic clippy requires `# Errors` sections the comment policy does not sanction.
- 🔵 **Standards**: Module docs in `resolve.rs` and `next_number.rs` go stale.
- 🔵 **Standards + Compatibility**: Unreleased changelog entries keep naming the retired verbs.

Suggestions:

- 🔵 **Compatibility**: Re-key under literal-text patterns changes too; say so in the changelog.
- 🔵 **Security**: `render` pads to any width; `validate` should report `WidthOverflow`.
- 🔵 **Usability**: Help should say which `work id` verb to use and that `check`'s output feeds `--id`.
- 🔵 **Usability**: Echo an exact ID whose key differs from `work.key` before writing.
- 🔵 **Code Quality**: Consolidate all of `allocation_message`, not just the overflow text.
- 🔵 **Standards**: Phases 3 and 7 omit the `work` public-API fixture step.
- 🔵 **Standards**: The corpus guard belongs in `tasks/lint/`, wired into `cli:check`.
- 🔵 **Standards**: The `work id` modules depart from corpus-cli's one-module-per-noun layout.

### Assessment

The `--id` feature design is now sound and most of the first review is
closed. The plan still needs another revision before implementation, for
seven major issues, all introduced by the relocation phases rather than by
`--id` itself. Four of them are design gaps with a clear fix: validate the
pattern and key inside scheme resolution, retire `IdScanner` in Phase 4, put
the requested-ID source in `work` so `create_local_item` can take it, and
decide `canonicalise-id`'s refusals. The other three are behaviour changes
the plan must either prevent or pin: slug widening, the key-run limit now
reaching allocation, and the moved tests' contradicted expectations. The
minor findings cluster around the visualiser's join keys, migrate's ports,
message wording and verification commands.

## Re-Review (Pass 3) — 2026-10-30

**Verdict:** REVISE

Of the 73 pass-2 findings, 53 are resolved, 19 partly resolved and 1 still
present. The validated grammar, the backtracking walk, the frozen legacy
linkage, the `LocalId` seam and the `identity_key` join closed the pass-2
majors. Eight new majors remain. Two are behaviour defects: rekey refusing
literal-text IDs, and the pre-existing run-plan path escape. Four are seams
the plan text leaves contradictory or underspecified. Two are missing red
tests for changes the plan makes.

### Previously Identified Issues

Architecture: 5 resolved; partially resolved — the pending-push seam (the
dry-run's read-only check is unspecified) and `holder_of`'s convergence
(deferred to 0302, which has shipped).

Code Quality: 8 resolved; partially resolved:

- `canonicalise-id` behaviour (its error map loses `PatternError`; `v1` is
  newly accepted);
- error-type overlap;
- the exact walk (Phase 6 still names the retired `segments`);
- `IdParts` and the taken rule (`RequestedId` duplicates `IdParts`);
- the `--project` parse site.

Test Coverage: 7 resolved; partially resolved — moved-test expectations
(visualiser duplicates and `normalise_id` ambiguity), malformed-pattern pins
(`read_scheme`, `work id next`, rekey, sync) and skill branches without an eval
(7.3 resume, Step 0 rename, `E_PUSH_PENDING`).

Correctness: 6 resolved; partially resolved — join keys (`id_in_filename`'s
output form), the dry-run pending-push inputs, and the key-run limit (a
digits-only literal after `{key}`).

Usability: 8 resolved; partially resolved — the mismatch fallback clause, the
welcome tip under `{tracker}`, and the key-differs note without `work.key`.

Compatibility: 6 resolved; partially resolved — `canonicalise-id`'s residual
changes; still present — Phase 5 does not regenerate the `work` API fixture.

Security: 3 resolved; partially resolved — the skill's create commands and
recovery-entered IDs lack the Step 0 guards.

Standards: 10 resolved; partially resolved — stale docs, including a false
`pup.ron` rewrite.

### New Issues Introduced

Major:

- 🟡 **Correctness + Test Coverage**: IDs rendered with literal text become `Unclassifiable`, so `work rekey` refuses — `rekey::sequence_of` reads only all-digit or `<key>-<digits>` IDs; once Phase 3 allocates `v001`, `PPv001` or `PP_0001`, every rekey run refuses, and the Phase 4 changelog's promise does not hold. No test covers the fix.
- 🟡 **Security**: A rekey run plan's renumbered targets reach file paths unvalidated — `run_plan_store.rs:313` decodes them raw and `retirement.rs:373-399` joins them into paths, so a committed run plan can create files outside the work directory. Pre-existing, but Phase 4 rewrites this decoder.
- 🟡 **Code Quality + Compatibility**: The scheme-reading steps return `kernel::Error`, losing the typed `PatternError` — canonicalise's `E_PATTERN*` map, compose's `ComposeError::Pattern` and create's message each need it; `read_scheme` also prints the deprecation notice inside a read.
- 🟡 **Code Quality + Test Coverage**: `normalise_id` is ported and re-tested with no caller — Phase 5 replaces its only caller with `identity_key`, and the visualiser's duplicate tests are not listed.
- 🟡 **Code Quality + Architecture + Correctness**: The pending-push precondition has three call sites but no shared function, and the dry-run lacks its inputs (`work.integration`, the rendered body for a `Created` marker).
- 🟡 **Code Quality + Architecture + Test Coverage + Correctness**: Where `--project` is parsed is stated three ways — `locate_work_items` cannot see it; Phase 3, Phase 6 and Error Identifiers imply different refusal orders for `--id 0400 --project 1X`.
- 🟡 **Test Coverage**: Scheme-time validation has no red test where it lands — `read_scheme`, `work id next`, rekey and sync have no refusal test; the Phase 3 and 7 create pins pass before the change.
- 🟡 **Test Coverage + Code Quality + Compatibility**: `WorkItemFile::content: Option<String>` and `.MD` listing reach about 20 consumers, but only the listing is tested; `work list` gains a warning and writers' `None` handling is unspecified.

Minor:

- 🔵 **Architecture + Compatibility + Correctness + Test Coverage**: The migrate port sends m0009 and m0002 through the live, stricter grammar, so they error on configs they process today.
- 🔵 **Compatibility + Correctness + Architecture**: `read_scheme` must parse `work.key` only under a `{key}` pattern, as today; the domain type should hold that invariant.
- 🔵 **Correctness + Test Coverage**: `id_in_filename`'s output form is unspecified, so the `identity_key` invariant can fail (unpadded filenames, `_` separators, literal text).
- 🔵 **Compatibility + Correctness**: The visualiser would serve the folded join key as each item's displayed `workItemId`, and `[[WORK-ITEM-42]]` would stop resolving.
- 🔵 **Compatibility + Correctness + Usability**: `work list` swallows the new refusals and falls back to the numeric scheme; the changelog names too few commands; newly refused forms print raw characters and no setting.
- 🔵 **Architecture + Code Quality**: `LocalId::Requested` lacks the scheme `refuse_if_taken` needs; `RequestedId` duplicates `IdParts`; the taken rule is still split.
- 🔵 **Architecture + Code Quality**: `IdConvention` is wider than linkage needs, and `LegacyNumericIds` half-implements it.
- 🔵 **Architecture**: `holder_of`'s convergence is deferred to 0302, which has shipped.
- 🔵 **Architecture**: Phase 9 is entangled with Phases 3 and 7, though declared independent.
- 🔵 **Architecture + Test Coverage**: The convention comparator in `work` cannot reach `RegexScanner`; the tracker filename pins have no home after Phase 5; the guard's vocabulary is narrow.
- 🔵 **Architecture + Standards + Code Quality**: The `pup.ron` rewrite would be false; several module docs go stale.
- 🔵 **Standards + Code Quality**: Phase 6 still names `segments` and `scheme.id_pattern`, which earlier phases removed.
- 🔵 **Standards + Compatibility**: Phase 5 omits the `work` API fixture; Phases 3 and 6 under-list their diffs.
- 🔵 **Code Quality**: Error types still overlap (`MissingKey` in three enums; `IdKey::parse` returns the whole `PatternError`).
- 🔵 **Code Quality**: `WithProject` admits impossible states; `PatternKeying` repeats what the pattern knows.
- 🔵 **Security + Correctness**: The O(n²) walk bound depends on one key and one number, which the grammar does not enforce.
- 🔵 **Security**: The skill's create commands and recovery-entered IDs lack the `--` and `'` guards.
- 🔵 **Usability**: The mismatch fallback describes a key and text that `{number:04d}` lacks.
- 🔵 **Usability**: The skill compares an exact ID's key with a `work.key` it never loads.
- 🔵 **Usability**: The welcome tip shows under `{tracker}`, and a reply in its form is not parsed.
- 🔵 **Usability**: `E_ID_TRACKER_OWNED` reads as though the item was saved.
- 🔵 **Usability**: `/create-work-item` treats `work resolve`'s malformed-pattern refusal as "not an ID".
- 🔵 **Usability + Test Coverage**: **Allocate automatically** has no resume point after drafting; several resume points have no eval.
- 🔵 **Standards**: Phase 1's nextest filter matches no test, and its test departs from the `tracker` crate's layout.
- 🔵 **Standards**: The corpus guard lacks the `test_mise.py` placement pin and the README list update.
- 🔵 **Standards + Compatibility**: A Breaking entry for renaming unreleased verbs departs from precedent; the lock entry's "older `work next-number`" should keep its name.
- 🔵 **Test Coverage**: The whole-string fallback in `holds` is unreachable from `refuse_if_taken`.
- 🔵 **Test Coverage + Usability**: The retired-verb pointers are untested with `--count` or `--help`.

Suggestions:

- 🔵 **Standards**: Phases 3 and 5's targeted commands skip tests those phases add.
- 🔵 **Code Quality**: Name the ID readings and predicates as pairs (`read`, `read_exact`, `read_filename`; `is_own_key_id`, `is_any_key_id`).
- 🔵 **Usability**: Step 0 should apply Step 5's re-key and busy-lock handling.

### Assessment

The `--id` design itself is settled and its findings are closed. The plan
does not converge, because each revision widens what the relocation touches,
and every widening surfaces another seam: rekey, run plans, migrations,
visualiser display, the shared listing. The remaining majors fall into three
groups. Rekey under literal-text patterns and the run-plan path escape are
real defects to fix. The precondition, `--project`, error-typing and
`normalise_id` seams need one clear decision each. Validation and the
listing change need red tests. Much of the minor list is plan-text
consistency after many edits. A single careful editing pass would clear
most of it.

## Re-Review (Pass 4) — 2026-10-30

**Verdict:** REVISE

The one-pass rewrite closed most of pass 3. Of pass 3's eight majors, six
are resolved and two are partly resolved: the literal-text re-key gap, and
the shared pending-push precondition, which is now a minor. Four majors
remain. One is a re-key gap three lenses found. Two are defects in the
grammar's lenient reading and in Step 0's new stderr test. The fourth is a
visualiser lookup the plan does not list. The minors are mostly seams the
plan names but does not specify, and plan-text consistency.

### Previously Identified Issues

Architecture: 8 resolved; partially resolved — the shared pending-push
precondition (the dry-run re-decides its arms), the taken rule (re-key's
`linker_of` still compares `external_id` whole), and the `work` `pup.ron`
comment (stale again after Phase 5).

Code Quality: 12 resolved; partially resolved — the `--project` parse site,
the pending-push precondition, the `pup.ron` comment and stale docs,
`PatternKeying` (a third public reading), and naming the readings as pairs.

Test Coverage: 12 resolved; partially resolved — moved-test expectations
(visualiser `config.rs:355-425` and Phase 2's break of the visualiser's
`tests/parity.rs`) and literal-text re-key (the tests stop short of
`classify`).

Correctness: 8 resolved; partially resolved — `id_in_filename`'s legacy
bare-digit fallback, the key-run limit (a digit literal after `{key}`), and
literal-text re-key.

Usability: 9 resolved; partially resolved — the key-differs note (the
injected **Local ID prefix** ignores the deprecated alias).

Compatibility: 10 resolved.

Security: 3 resolved.

Standards: 6 resolved; partially resolved — stale docs and the `pup.ron`
comment.

### New Issues Introduced

Major:

- 🟡 **Architecture + Correctness + Test Coverage**: Re-key reads pre-change IDs through the post-change pattern — `sequence_of` and `project_key_of` read through the configured (target) pattern, so moving `v001` or `PP_0001` onto `{key}-{number:04d}` gives `Unclassifiable` and the run refuses. Phases 3 and 4 together regress this, and the planned tests reach `is_in_target_shape` but not `classify` (`rekey/classify.rs:29`) or `numeric_mapping` (`rekey/mapping.rs:54`).
- 🟡 **Correctness**: The taken check misses an exact duplicate when a digit literal follows `{key}` — under `{key}1{number:04d}`, `read_exact` reads `--id PP10100` as (`PP`, 100), while `IdParts::parse` reads the held `PP10100` as (`PP10`, 0). The same split feeds `holder_of_under`, `sequence_of`, `identity_key` and `canonicalise`.
- 🟡 **Usability + Test Coverage**: Step 0's start-of-stderr `E_RESOLVE_INVALID:` test fails under the deprecation warning — `work resolve` prints the `work.default_project_code` warning first (`resolve.rs:86`, `config.rs:80-85`), so every plain topic stops in such repositories. No eval covers the topic branch continuing.
- 🟡 **Compatibility + Test Coverage**: `related.rs:59-60` queries the identity-keyed reference maps with the displayed ID — `/api/related` drops `declaredInbound` for `pp-0500` or `PP-42` items; `related.rs`, `work_item_id_from_entry`'s callers and the `IndexEntry` `ClusterEntry` impl are not listed.

Minor:

- 🔵 **Code Quality**: Phase 7 adds `refuse_if_requested_id_taken` with no caller until Phase 8; `accelerator-work` is binary-only with `warnings = "deny"` (`cli/Cargo.toml:260-261`), so Phase 7 cannot build green.
- 🔵 **Code Quality + Architecture + Standards**: The `Requested` arm names a `Creation::work_item_files()` that does not exist (`Creation` reads `identities()`, which drops unreadable files), leaves the `RequestedId` hand-off from `creation_context` unspecified, and flattens the typed refusal inside a `String`-returning adapter with no stated owner for `relative_to`.
- 🔵 **Code Quality + Architecture + Standards**: The dry-run's `refuse_unfinished_push` re-reads the marker and re-decides `work::sync`'s mapping (`push_precondition.rs:66-95`), and the adapter's new `push_precondition` shares the domain function's name.
- 🔵 **Correctness + Test Coverage + Usability**: Phase 8's `Created`-marker rationale is wrong — after `CreatedUnwritten` the ID is untaken, the dry-run previews, and a changed body makes the real `--push` create refuse with `E_PUSH_FINGERPRINT_MISMATCH`; the `push: false` rows contradict Phase 7's local-create pin; and 7.1 sends `E_PUSH_PENDING` from the dry-run to the generic abort with a "re-run" line.
- 🔵 **Code Quality**: `--project` key selection moves out of `work::next_number::allocate` into two `work-cli` copies, and an invalid flag is typed as `SchemeError`, a config-reading error.
- 🔵 **Code Quality**: `WorkItemIdScheme::new` as "the one constructor" holds only with private fields; 55 struct literals in 18 files build schemes today.
- 🔵 **Code Quality + Standards**: `IdPatternError` keeps a name beside `PatternError` while holding only the tracker-integration rule; `work` has no `thiserror`, so `#[from]` on `AllocationError` needs a hand-written `From`.
- 🔵 **Code Quality + Architecture + Test Coverage**: The visualiser validates the scheme in `compose`, flattens it to strings, then must parse it again in `WorkItemConfig::from_raw` (`server.rs:79`) with no named error; the `from_raw` tests at `config.rs:355-425` assert the deleted `InvalidScanRegex`.
- 🔵 **Compatibility**: The served `clusterKey` changes form (`pp-0500` → `PP-0500`), and an other-key work item gains a slug under `slug_start`; the plan says neither reaches the frontend.
- 🔵 **Architecture**: Re-key's `linker_of` (`collision.rs:41-45`) compares `external_id` whole, so re-key can renumber onto `PP-0905` beside an item linked to `PP-905`, which `work create --id` refuses.
- 🔵 **Correctness + Test Coverage**: Legacy bare-number IDs under a keyed pattern read inconsistently — `42-x.md`'s fallback join key is unstated against `id: "0042"` (today `PROJ-0042`), and a legacy `0400` holder leaves `PP-0400` untaken while re-key renders it to `PP-0400`.
- 🔵 **Correctness**: `read_exact`'s non-zero rule leaks into `is_in_target_shape` and the run-plan decoder, so an `id: "0000"` item is renumbered onto itself and a `PP-0000` target decodes as `Malformed`.
- 🔵 **Correctness + Security**: The 255-byte cap applies to whole filenames in `parse_prefix`, so a long non-ASCII slug drops its holder from allocation and the taken check; bound the walk, not the input.
- 🔵 **Security**: Error reasons still echo raw config substrings after the escaped prefix (`KeyError::BadKeyValue`, text inside braces in `UnknownToken` and `BadFormatSpec`).
- 🔵 **Security**: Non-ASCII control, bidi and format characters pass the hostile set, contradicting "`read_exact` admits ASCII only".
- 🔵 **Security**: Run-plan `write_set` and fingerprint paths are still decoded raw (`run_plan_store.rs:141-143,381-409`), so the `### Security` changelog line overclaims.
- 🔵 **Compatibility**: m0002's deviation test uses `x'{number:04d}`, which m0002 never canonicalises (`m0002.rs:51-53`), and the `MY_PROJ` pin is vacuous.
- 🔵 **Compatibility**: Newly refused configurations need a `### Migrations` entry with the remedy (precedent `CHANGELOG.md:778-783`); the Phase 5 visualiser entry lists refusals `compose.rs:212` already makes; the bare-`{number}` slug entry belongs in Phase 5; `work resolve` already lists `.MD`.
- 🔵 **Compatibility**: `work next-number` prints failures without `Error: ` (`main.rs:451-453`), contradicting a Phase 4 expectation; canonicalise's text after the code is unspecified.
- 🔵 **Standards + Compatibility**: The verb-rename changelog list misses `:422`, `:425`, Phase 3's `### Changed` entry and both Phase 4 entries.
- 🔵 **Standards + Code Quality**: Stale docs remain — `corpus/src/lib.rs:1-7`, `corpus-adapters/src/lib.rs:3-4`, `slug.rs:43`, `filesystem.rs:62-63`, `creation.rs:622-628`, `indexer.rs:3392` — and the `work` `pup.ron` comment goes stale in Phase 5.
- 🔵 **Standards**: The guard's wiring omits the invoke collection (`tasks/__init__.py:98-109`) and `__all__`.
- 🔵 **Standards**: `exit_codes.rs` has no `E_WORK_KEY_REQUIRED` entry to name `E_WORK_KEY_INVALID` beside.
- 🔵 **Standards + Code Quality**: Phases 4 and 7 name `work id next` and `work id check` before they exist; the `resolve_scheme` caller list omits `resolve`, `promote` and sync authoring; eval 49's name says `next_number`.
- 🔵 **Usability**: Refusal examples render number 1, so `WithProject` suggests writing a key the ID already has, offers allocation under an invalid `--project`, and `0001` is usually taken.
- 🔵 **Usability**: The **Local ID prefix** injection ignores the deprecated `work.default_project_code` alias.
- 🔵 **Test Coverage**: The deprecation print moves to every caller but only two commands test it.
- 🔵 **Test Coverage**: `work sync`'s widened `id_is_token_safe` gate has no keyed-scheme test.
- 🔵 **Test Coverage**: The per-eval `pass_rate` 1.0 floor is enforced by nothing.

Suggestions:

- 🔵 **Code Quality**: A private `correction_for` shared by `admit` and `propose`; name the five ID projections for their roles.
- 🔵 **Usability**: `work id check`'s help overstates parity with `work create --id` for correctable requests; the welcome tip's `--id 0400` fails under literal-text patterns; say that allocation continues above a requested ID.
- 🔵 **Test Coverage**: Move Phase 1's original `corpus` cases over unchanged; test compose under `{tracker}` with no integration.
- 🔵 **Standards**: Phase 8's targeted command misses the inverted `cli.rs` test.

### Assessment

The plan is converging: majors fell from 8 to 4, and every lens resolved
most of its pass-3 items. Two majors need a decision, because both come
from the lenient reading. Re-key needs a reading of existing IDs that does
not depend on the target pattern, and the grammar must either refuse a
digit literal after `{key}` or prefer the exact-width split. The other two
majors, `related.rs` and the stderr line test, are targeted fixes. Most
minors are unspecified seams (`Creation`'s listing, the `RequestedId`
hand-off, the visualiser's second parse) and text consistency: changelog,
stale docs, verb names. Phase 7's uncalled function is a build failure
under `warnings = "deny"` and should be fixed with the majors.

## Re-Review (Pass 5) — 2026-10-31

**Verdict:** REVISE

All four pass-4 majors are resolved, and every lens resolved all or nearly
all of its pass-4 items. Three majors remain, each introduced by a pass-4 fix
and each a point fix rather than a seam: re-key's one-digit-run fallback
reads draft IDs, Step 0's narrowed rule stops a topic in a repository with
no work directory, and `admit` and `correction_for` call each other. The
minors centre on one seam four lenses found (the taken check's filename
reading lacks the legacy rule) and on plan organisation: test placement,
fixture regeneration and a missing phase edge.

### Previously Identified Issues

Architecture: 8 resolved.

Code Quality: 13 resolved; partially resolved — naming the readings as
pairs, and the ID projections (`extract_id` still overlaps
`id_in_filename`).

Test Coverage: 10 resolved; partially resolved — `related.rs` (the
visualiser's refresh path is untested) and legacy bare numbers (the filename
reading).

Correctness: 8 resolved; partially resolved — legacy bare numbers (the
taken check's filename reading and allocation skip the legacy rule).

Usability: 6 resolved.

Compatibility: 6 resolved.

Security: 4 resolved.

Standards: 9 resolved; partially resolved — Phase 4's `identity_key` bullet
still names `work id canonicalise` before Phase 9.

### New Issues Introduced

Major:

- 🟡 **Correctness + Test Coverage**: Re-key's one-digit-run fallback reads draft IDs — `default_key_floor` (`mapping.rs:105-126`) sends every `id` and alias through `sequence_of`, so `draft-ab1234` sets the fresh-number floor to 1234 and `draft-a12345` exhausts the cap. A `{tracker}`-to-numeric re-key renumbers by random suffixes or refuses. The fallback also flips `sequence_of("-0042")` (`sequence.rs:89-95`) from `None` to 42 unlisted.
- 🟡 **Usability + Test Coverage**: Step 0 stops a plain topic in a repository without a work directory — `work resolve` canonicalises the directory before classifying (`resolve.rs:86-88`), so a fresh repository exits 1 with no `E_RESOLVE_INVALID:` line, and the skill now stops where it drafted before. No CLI test pins the stderr contract Step 0 relies on.
- 🟡 **Code Quality**: `admit` and `correction_for` recurse with no base case — `admit` fills its correction from `correction_for`, which requires its rendering to pass `admit`, so `--id 0` and `--id 0000` loop.

Minor:

- 🔵 **Architecture + Code Quality + Test Coverage + Correctness**: The taken check's filename reading skips `holds`' legacy rule — `refuse_if_taken` reads filenames through `parse_prefix` and `same_id`, so a legacy `0400-x.md` with no frontmatter, or unreadable, never takes `PP-0400`, contradicting `a_legacy_number_takes_the_configured_keys_id`; allocation skips legacy holders too, and `an_unparseable_held_value_never_takes_a_requested_id` still lists `id: "0400"`.
- 🔵 **Correctness**: Re-key's `linker_of_under` blocks re-keying this repository onto its tracker key — `0021-artifact-persistence-lifecycle.md` links `PP-43` beside `0043-*`, so `{key}-{number:04d}` with key `PP` refuses with a collision and no remedy.
- 🔵 **Correctness**: `is_in_target_shape` "through `read_exact`" drops today's key check (`XYZ-0002` under `ACC` would be in shape) and makes literal case a rename (`V001` → `v001` aborts on a case-insensitive filesystem, `retirement.rs:312-313`).
- 🔵 **Correctness**: `WithProject`'s example reads "the request's number", but no named reading reads a keyless `0400` under `{key}-{number:04d}`, contradicting the test's `XY-0400`.
- 🔵 **Architecture**: Phase 10 now depends on Phase 8 (`refuse_if_requested_id_taken`), which the diagram does not show.
- 🔵 **Architecture + Code Quality**: The dry-run "runs checks 1 to 5 before reading `work.integration`", yet reads it between checks 4 and 5, and `run_with` reads it again with a different error policy (`create.rs:651-656`).
- 🔵 **Architecture + Code Quality**: The visualiser keeps `RawWorkItemConfig`'s strings beside a `#[serde(skip)]` scheme, so fixtures such as `make_cfg_with_key` (`indexer.rs:1891-1913`) can silently boot under the default pattern.
- 🔵 **Code Quality**: The pending-push seams don't fit the code: `check_pending_push` must turn `Refuse` into the error, `read_push_marker` cannot return a borrowed `MarkerState<'a>` (`push_precondition.rs:37-41`), and the `Requested` arm reads the corpus twice.
- 🔵 **Code Quality**: `extract_id` moves into `work` with no production caller beside `id_in_filename`.
- 🔵 **Code Quality**: The visualiser's `canonicalise_refs` and `canonicalise_one_id` (`indexer.rs:1156-1191`) will produce identity keys under canonicalising names; four comments there go stale.
- 🔵 **Test Coverage**: The visualiser's refresh path (`indexer.rs:1017-1116,1217`) is re-keyed with no failing test.
- 🔵 **Test Coverage + Standards**: The per-eval 1.0 guard has no failing synthetic case, and belongs in its own module beside `test_boundary_evals.py`.
- 🔵 **Standards + Test Coverage + Architecture**: Phase 8 files the dry-run lock test and two pins under `work`'s `push_precondition.rs`, which cannot reach `run_with`.
- 🔵 **Standards + Compatibility**: Phase 8 makes `unfinished_push` public in the pinned `work` API without regenerating the fixture; Phase 4's expected diff omits `classify` and `sequence_of`'s new parameter.
- 🔵 **Compatibility + Standards**: Phase 4's changelog, `exit_codes.rs` paragraph and Migration Notes say every `work` command refuses a bad pattern, though `show`, `diff`, `update`, `template-hints` and `link-external-id` never read it, and `list` and canonicalise skip `E_WORK_KEY_REQUIRED`; Phase 9 does not rename `work canonicalise-id` in that paragraph; the remedy wording does not fit every refused form.
- 🔵 **Standards**: `PushingRepo` is private to `create.rs`'s tests (`create.rs:811`), so Phase 10's tests cannot use it.
- 🔵 **Standards**: New listing and run-plan tests sit away from their suites (`tests/work_item_files.rs`, `tests/rekey_run_plan_store.rs`).
- 🔵 **Standards + Code Quality**: `work-cli`'s `resolve_scheme` and `read_scheme` wrappers share the adapter functions' names but add printing.
- 🔵 **Standards**: `migrate-cli`'s new dependency on `work` is not listed.
- 🔵 **Usability**: "Later automatically numbered items continue above it" is wrong for another key or a back-filled gap; a `corrected` ID under another key gets no key-differs note.
- 🔵 **Usability**: The welcome tip points at an **ID pattern** the user cannot see.
- 🔵 **Usability**: `E_PUSH_MARKER_UNREADABLE` is routed to pending-push handling the skill does not have for `--id`.
- 🔵 **Security**: The 255-byte filename window's cut point is unspecified; a naive cut panics mid-character.
- 🔵 **Security**: Run-plan `old` IDs are still decoded raw (`run_plan_store.rs:323`), so a tampered plan is applied partway before the sink refuses.
- 🔵 **Security**: Pattern literals may hold `$`, a backtick, `;`, `&`, `(`, `)` and spaces, which Phase 3 now writes into filenames that skills pass unquoted.

Suggestions:

- 🔵 **Code Quality**: One `existing_id` reading for re-key's number and key; rename the printing wrappers.
- 🔵 **Test Coverage**: Pin that the dry-run reads no marker without `--push`.
- 🔵 **Usability**: `work id check`'s "re-run that create to recover it" leads to a loop; point to `/sync-work-items`.
- 🔵 **Security**: Escape the holder path and `highest_file` in `E_ID_TAKEN` and `E_PATTERN_OVERFLOW`.
- 🔵 **Compatibility**: Phase 5's changelog understates newly served IDs (`X1-0400`) and clusters (`v001`); one Migration Notes line misstates m0002.
- 🔵 **Standards**: Add `id_pattern` to Phase 5's command; phrase changelog edits without plan phases, and fold Phase 3's refusal entry into Phase 4's.

### Assessment

The plan is converging: majors fell from 8 to 4 to 3, and every pass-4 major
is closed. The three remaining majors are each fixed in a sentence: exclude
draft IDs from the fallback, classify `work resolve`'s input before reading
the directory, and give `correction_for` its own exact-shape predicate. The
one recurring seam is the taken rule's filename reading, which a named
scheme reading with the legacy rule closes for both the taken check and
allocation. Two minors need a decision: whether re-key compares
`external_id` by key and number (this repository's own `PP-43` link would
block a re-key), and whether pattern literals exclude shell-active
characters. The rest is plan organisation and wording.

## Re-Review (Pass 6) — 2026-10-31

**Verdict:** REVISE

All three pass-5 majors are resolved, and every lens resolved all or all but
one of its pass-5 items. One major remains, rated at medium confidence: the
narrowed literal set refuses the `{{`/`}}` escape that two parity rows,
the configure skill's pattern reference and an `[Unreleased]` changelog
entry still treat as valid. It needs one decision. The minors are local:
two new refusals without named codes, the taken rule under `{tracker}`, the
dry-run's integration read, and test naming, placement and wording.

### Previously Identified Issues

Architecture: 4 resolved; partially resolved — the dry-run's integration
read.

Code Quality: 7 resolved; partially resolved — the dry-run's integration
read, and the visualiser's stale identity comments.

Test Coverage: 9 resolved.

Correctness: 6 resolved.

Usability: 5 resolved.

Compatibility: 3 resolved.

Security: 4 resolved.

Standards: 9 resolved; partially resolved — Phase 4's expected public-API
diff.

### New Issues Introduced

Major:

- 🟡 **Compatibility + Security + Standards**: The `{{`/`}}` escape is both refused and kept — literal text is now letters, digits, `.`, `_` and `-`, so an escaped brace is `HostileChar`, yet the parity rows `{{x}}{number}` and `{key}{{}}{number}` must parse, the configure skill's DSL reference and validation rules (`skills/config/configure/SKILL.md:715,729-743`) still document the escape and only rule 2's characters, and `CHANGELOG.md:365` says `{{tracker}}` is a literal. Kept, braces make `{1..3}0001-x.md` brace-expand unquoted.

Minor:

- 🔵 **Compatibility + Standards**: Re-key's linked-key clash and retirement's path-component refusal have no named variant, code, keyword or exit status, though `keyword_exit_codes.rs` freezes both contracts; Phase 4's expected diff omits `refuse_collisions`' scheme and a fallible `recovery_dir`.
- 🔵 **Correctness + Architecture**: The taken rule under `{tracker}` is unstated; comparing tracker keys by number would block a re-key to `{tracker}` and promotion beside an alias such as `PP-0760`. `id_in_filename`'s `{tracker}` bare-digit reading sits outside `read_held_prefix`.
- 🔵 **Code Quality + Architecture**: The dry-run reads `integration_state` "as an Option", losing the missing-integration text, needing a root it has not discovered, and possibly changing the plain dry-run.
- 🔵 **Security**: `existing_id`'s fallback reads an `id` such as `a/b/0001`, which the plan's own decoder then refuses, leaving `E_REKEY_PLAN_UNREADABLE` that blocks identity commands; restrict it to one normal path component.
- 🔵 **Usability**: The 7.3 no-re-run branch omits `E_PUSH_FINGERPRINT_MISMATCH` and `E_PUSH_ALREADY_WRITTEN`.
- 🔵 **Usability**: The numbering note names a key the default pattern lacks; the clash remedy omits `{tracker}` as a route; `Unclassifiable`'s text still blames "no numeric or tracker shape"; `work id check`'s help leaves the third field and refusals ambiguous.
- 🔵 **Code Quality**: `make_cfg_with_key` loses its key to `WorkItemIdScheme::new`; more stale indexer comments and `canonicalise_refs_*` test names; `announced_scheme_for_writing` mislabels read-only callers; `RequestedId` cannot report `MissingKey`.
- 🔵 **Test Coverage**: The visualiser's `canonicalise_tests` change expectations not written first; the per-eval guard passes vacuously; no test pins that a missing integration fails after the `--id` checks.
- 🔵 **Compatibility**: Allocation counting legacy numbers belongs under `### Breaking`, as Phase 3's allocation entry is; `work resolve`'s new exit codes in a fresh repository go unannounced.
- 🔵 **Standards**: Phase 1 contradicts the `tracker` crate's "holds no logic" docs; three new tests carry the retired verb's name; Phase 4's and Phase 5's targeted lists miss `next_number` and `api_related`.

Suggestions:

- 🔵 **Code Quality + Correctness**: Expose `correction_for`'s walk; `WithProject`'s example can render number 0; share one path-component predicate.
- 🔵 **Security**: Escape the requested ID in `E_ID_PATTERN_MISMATCH` and the filename in the clash message.
- 🔵 **Correctness**: The dry-run gives `E_ID_TAKEN` where the real create gives `E_PUSH_ALREADY_WRITTEN`.
- 🔵 **Test Coverage**: Pin `highest_file`'s escaping.
- 🔵 **Compatibility + Standards**: Name `work id check` in the lock changelog entry; fix two cross-references.

### Assessment

The plan has converged: majors went 8, 7, 8, 4, 3 and now 1, and no
pass-5 finding is still present. The remaining major is a choice already
implied by the approved literal set: refuse escaped braces, move the two
parity rows to expected values, and update the configure skill's reference
and validation rules. The minors are each a sentence or a test. Another full
eight-lens pass would mostly find more of this size; after these fixes, a
narrow check by the correctness, compatibility and standards lenses would
be the proportionate next step.

## Re-Review (Pass 7, narrow) — 2026-10-31

**Verdict:** COMMENT

A narrow check by the correctness, compatibility and standards lenses. The
pass-6 major, escaped braces, is resolved, and no new major was found. The
remaining findings are local minors, mostly at the edges of the pass-6
edits; none needs a new design decision.

### Previously Identified Issues

The escaped-brace major is resolved: the grammar, parity rows, configure
reference, `CHANGELOG.md:365` and the `### Migrations` entry agree. The
`{tracker}` taken rule, the dry-run's integration read, the test naming and
placement items, the changelog filing and the escaping items are resolved.
Partially resolved: the new refusals (the promotion keyword is still
unnamed), the expected public-API diff, and `existing_id`'s restriction
(`tracker_mapping` still saves an old ID unchecked).

### New Issues Introduced

Minor:

- 🔵 **Compatibility + Standards + Correctness**: `RetirementRefusal::IdNotOnePathComponent` needs its own `not-promoted` keyword and `FROZEN_NOT_PROMOTED` row, since `RetirementRefusal::keyword()` gives each variant its own (`retirement.rs:208-215`); as written, `every_not_promoted_reason_maps_to_its_frozen_exit_code` fails. `exit_codes.rs:105-118` and the `/sync-work-items` reason table need the keyword, and re-key's `refused_before_writing` must map it to a nothing-written code.
- 🔵 **Compatibility + Correctness**: Making `recovery_dir` fallible ripples into `RetirementRecord::of` and four message-building callers; refusing in `plan_retirement` and at record decoding keeps it infallible.
- 🔵 **Compatibility + Standards + Correctness**: Moved `corpus` tests still assert escaped braces are valid (`work_item_id.rs:495-498,569-587`); one parity line says the compiler accepts `{{/}}{number}`, which it refuses for `/`; Phase 4 names Phase 3 counterparts Phase 3 never lists.
- 🔵 **Compatibility + Standards**: Braces are refused from Phase 3, but the configure reference and `CHANGELOG.md:365` are corrected only in Phase 4.
- 🔵 **Standards + Correctness**: `Unclassifiable` carries only a path, yet its new message needs an example and fits only one of its four causes (ambiguous digit runs, no number, characters outside the ID set, an `external_id` that is not a tracker key).
- 🔵 **Correctness**: The linked-collision remedy blames `work.key` for clashes it did not cause (a duplicate link under `{tracker}`, a keyless pattern); use it only when the `external_id` reads under `work.key`.
- 🔵 **Correctness**: `existing_id`'s character restriction blocks the `### Migrations` route for IDs such as `x'0001` or `PP 0001`; restrict to one normal path component with no control characters instead.
- 🔵 **Correctness**: With `--push` and no integration, the dry-run reports `E_ID_TAKEN` where the real create reports the missing integration before check 5.
- 🔵 **Standards**: `a_tracker_key_is_compared_whole_under_tracker` is filed with the ID-reading tests rather than the taken-rule tests.
- 🔵 **Compatibility + Standards**: Phase 9's rename list misses Phase 4's `### Breaking` entry and points at a folded Phase 3 entry.

Suggestions:

- 🔵 **Standards**: The `docs-site` reference pages are generated and gitignored, so "regenerate" should not read as a committed change.

### Assessment

The plan has converged. Majors went 8, 7, 8, 4, 3, 1 and now 0. The
remaining minors are each a sentence, a keyword or a test row, and can be
folded in before implementation without another review pass; test-driven
implementation will surface anything smaller.
