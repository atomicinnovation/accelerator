---
type: plan-review
id: "2026-08-30-0221-canonical-frontmatter-quoting-standard-review-1"
title: "Plan Review: Canonical Frontmatter Quoting Standard"
date: "2026-08-30T15:24:05+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-08-30-0221-canonical-frontmatter-quoting-standard"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, correctness, code-quality, test-coverage, safety, standards, compatibility]
review_number: 1
review_pass: 3
tags: [frontmatter, corpus, validator, migration, quoting]
last_updated: "2026-08-30T17:43:25+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Canonical Frontmatter Quoting Standard

**Verdict:** REVISE

The plan is structurally excellent — a genuine single-choke-point emitter, a
migration that reuses that emitter as the single definition of "canonical"
rather than re-implementing quoting, deliberate phase ordering that keeps
`this_repositorys_own_corpus_is_clean` green at every merge, and unusually
disciplined TDD framing. Two critical defects block it: the Phase 3 validator
predicate as sketched is not quote-aware — it strips trailing comments and
splits commas over the raw value, which mis-flags a *real* canonical corpus
file today and re-creates the exact emitter/validator disagreement ADR-0065
exists to eliminate; and `m0008`, the first migration to fully re-serialise
frontmatter, silently drops inline comments and normalises CRLF on every
downstream plugin corpus it runs against, with the in-process self-check
structurally blind to the loss. Alongside these, several major issues concern
the enforcement proxy's rigour (weak AC #6 substring match, ineffective
idempotency test, understated template-shape parity) and concrete factual
errors in success-criteria commands (wrong `mise` lanes, invented public-api
snapshots).

### Cross-Cutting Themes

- **The one-standard-two-encodings risk is real and already broken** (flagged
  by: Correctness, Compatibility, Architecture, Code Quality) — ADR-0065's
  whole premise is a single predicate driving both emit and validate. The
  emitter is structural (`DoubleQuoted` arms) and the validator is lexical
  (`is_canonical_scalar`), and the plan's sketched validator diverges from the
  emitter on comment-bearing and comma-bearing quoted values. This surfaces
  concretely (Correctness's critical: a real corpus file breaks the self-check)
  and structurally (Architecture's major: nothing binds the two encodings, so
  it can recur). The fix must make the validator quote-aware *and* add a guard
  that holds emitter↔validator symmetry over arbitrary value trees, not just
  the corpus sample.
- **`m0008` ships to downstream corpora, not just this repo** (flagged by:
  Safety, Compatibility) — the plan reasons about comment/CRLF-losslessness
  only for *this* repo's measured-clean corpus, but the migration runs in every
  plugin user's `accelerator migrate`. There it is a silent, unvalidated,
  lossy rewrite of any commented or CRLF frontmatter.
- **Enforcement rigour lags the enforcement design** (flagged by: Test
  Coverage, Code Quality) — with no CI lane, AC #6/#7 are the only enforcement,
  yet the coverage test is a bare substring match, the idempotency check is
  ledger-gated (trivially true), and Phase 5 trades a ~30-case Python template
  suite for a single broken-template negative.

### Tradeoff Analysis

- **Field-agnostic simplicity vs verification strength for config** — the
  type-driven rule's great virtue is that config's untyped frontmatter conforms
  through the same emitter with no config-specific logic. The cost (Architecture,
  Safety): config is the one operationally critical write with *no* independent
  validator until 0227, resting solely on emitter correctness plus idempotency
  (a fixed point, not a proof of correctness). Recommendation: keep the deferral
  but add a Phase 2 round-trip semantic-equality assertion on a nested/block
  config fixture, so config is not the sole unvalidated write.
- **Producer-run enforcement reach vs testability** — validating in every
  plugin user's corpus as they work is the design's strongest property (per
  ADR-0065). The tradeoff is that a prose-driven skill's runtime honouring of
  the exit code is not testable, so AC #6/#7 are proxies. Accepted — but the
  proxies must be as strong as possible (bind AC #6 to the final section + a
  fenced block), which they currently are not.

### Findings

#### Critical

- 🔴 **Correctness / Compatibility**: Phase 3 validator predicate is not
  quote-aware — mis-flags canonical values and breaks the self-check
  **Location**: Phase 3 — `is_canonically_quoted` / `is_canonical_scalar`
  The sketch runs `strip_trailing_comment` over the *raw* value before parsing
  the quote and naive-splits flow arrays on `,`, neither respecting quote
  boundaries. `meta/research/codebase/2026-06-12-0108-local-docker-visual-regression-baselines.md:17`
  carries a real, correctly-quoted `last_updated_note: "Added follow-up
  resolving open question #1 — …"`; the predicate truncates at ` #1`, drops the
  closing quote, and emits `UNQUOTED-STRING` on a canonical value — turning
  `this_repositorys_own_corpus_is_clean` and the migration self-check RED,
  contradicting the Phase 2/3 "stays green" criteria. The comma branch mis-splits
  `["a, b"]` into `"a` and `b"`, and — coupled to Phase 1 deleting
  `needs_quoting` (which existed to quote `,`/`:`/`#` tags) — every such tag now
  round-trips as a quoted string this validator rejects. This is the exact
  emitter-emits/validator-rejects divergence 0221 exists to kill; it is latent
  in this repo only because the committed frontmatter happens to be `#`/comma-free.

- 🔴 **Safety / Compatibility**: `m0008` silently drops inline comments and
  normalises CRLF corpus-wide on downstream corpora
  **Location**: Phase 2 (the migration) / Migration Notes / What We're NOT Doing
  `m0008` is the first migration to fully re-serialise frontmatter (parse →
  render) rather than a surgical line rewrite like `m0006`/`m0007`. The `Yaml`
  value tree has no comment representation, so re-render unconditionally drops
  every inline frontmatter comment and normalises CRLF→LF across all files — and
  the in-process `validate_frontmatter` self-check is blind to it (a field with
  and without a trailing comment both validate). The plan scopes "drops comments
  by design" to *this* repo (measured comment-free), but the migration ships in
  the plugin binary and runs on every downstream user's corpus, where comments
  or CRLF may exist and would be erased with no diff or warning. AC #4's
  byte-identity anti-churn guarantee silently degrades to "canonical, comment-free,
  LF input only."

#### Major

- 🟡 **Architecture / Code Quality**: No binding symmetry guard between the
  emitter and validator encodings of the canonical rule
  **Location**: Implementation Approach; Phase 1 emitter vs Phase 3 validator
  The rule lives in two hand-maintained forms with nothing linking them at
  compile time, and only a single-sample corpus self-check to catch divergence.
  Because Phase 2 commits the corpus before Phase 3 tightens, a silent
  divergence surfaces only as a red self-check *after* the corpus is committed,
  forcing re-migration. Add a paired round-trip/property test that renders a
  representative set of scalar and sequence value trees through `document::render`
  and asserts the validator accepts every one (and rejects a bare mutation).

- 🟡 **Code Quality**: The general quoting check hardcodes a skip-list mirroring
  the dedicated checks
  **Location**: Phase 3 — `check_canonical_quoting`
  Skipping `id`/`schema_version`/`typed_linkage_keys` encodes "which fields have
  a dedicated check" in two unlinked places. A future dedicated check added or
  removed without updating the skip-list yields double-reports or a silent
  coverage gap. Derive the skip-set from one source both consult, and test that
  exactly one code fires per dedicated-check field.

- 🟡 **Test Coverage**: Render idempotency / AC #4 byte-stability lacks an
  effective automated test
  **Location**: Phase 2 (Manual Verification) / Testing Strategy
  The named real check — "re-running `accelerator migrate` reports `0008` as a
  no-op" — is ineffective: once `m0008` is in the applied ledger the engine
  skips it unconditionally, so the step is trivially true and never exercises
  the emitter's fixed-point behaviour. `this_repositorys_own_corpus_is_clean`
  checks validity, not byte-stability. Apply the transform twice to real-corpus-derived
  content and assert byte-equality on the second pass (mirror
  `migration_0007`'s byte-equivalence golden).

- 🟡 **Test Coverage**: Phase 5 template-shape port risks silent coverage loss;
  success criteria understate parity
  **Location**: Phase 5 (Changes #1, #3; Success Criteria)
  It deletes `test_template_frontmatter.py` (~30 cases, one negative per rule)
  but ports only the "seven checks with no Rust equivalent," implying
  base-field/type/schema_version/id/provenance are covered elsewhere — they are
  not, because `validate_file` structurally rejects templates. The criteria
  require only "a broken template fails" (2 of ~20 rules). Require full per-check
  parity with one negative per rule, and gate the deletion on that.

- 🟡 **Test Coverage**: AC #6 static coverage test uses a weak substring
  assertion for the sole enforcement mechanism
  **Location**: Phase 4 — Static coverage test
  A bare "contains a `corpus frontmatter validate` invocation" match passes even
  if the invocation sits in prose, a comment, or the wrong section — weaker than
  the sibling `test_skill_frontmatter_population.py` it models on, which
  heading-scopes each assertion. Bind it to the final persistence section and
  require the invocation inside a fenced bash block.

- 🟡 **Standards**: SKILL.md tool-permission lint attributed to the wrong `mise`
  lane
  **Location**: Phase 4 — Success Criteria
  "`mise run scripts:check`" checks only shfmt/ShellCheck/bashisms over the two
  thin-shell files; it never inspects SKILL.md frontmatter. The actual guard is
  `lint:skill-permissions:check` (Python, `tasks/lint/skill_permissions.py`).
  Following the plan literally gives false confidence the 21 new `allowed-tools`
  rules are valid.

- 🟡 **Standards**: Public-api snapshot handling is factually wrong
  **Location**: Phase 5 (Change #5); Phases 1/3/5 Success Criteria
  Phase 5 lists `corpus-adapters` and `corpus-cli` snapshots to regenerate, but
  per `tasks/public_api.py` only `corpus` is pinned — adapters and composition
  roots are exempt, so those files do not exist and `public-api:update` never
  generates them. Separately, snapshot verification/regeneration is attributed
  to `cli:check`, which does not run cargo-public-api; the actual commands are
  `public-api:update` / `public-api:check` (or the aggregate `check`).

#### Minor

- 🔵 **Correctness**: `is_bare_int` is unspecified — a bare numeric string on a
  field without a dedicated check (e.g. hand-edited `external_id: 750`) passes as
  a "bare integer," a false negative. Define its grammar; note the limitation.
  **Location**: Phase 3 — `is_canonical_scalar`

- 🔵 **Correctness**: A bare *malformed* timestamp double-reports
  `UNQUOTED-STRING` + `BAD-TIMESTAMP`, contradicting the plan's "compose without
  double-reporting" claim. Accept and document, or gate the format check on the
  quoting check passing first.
  **Location**: Phase 3 — timestamp composition

- 🔵 **Architecture / Safety**: Config conformance rests solely on emitter
  correctness + idempotency, with no independent checker until 0227 — the one
  operationally critical, unvalidated write. Strengthen the Phase 2 config test
  to a nested-mapping/block-sequence fixture with a round-trip semantic-equality
  assertion.
  **Location**: Phase 2 (config pass) / Migration Notes

- 🔵 **Architecture / Standards**: The Python retirement relocates the
  Rust→Python constant mirror (`BASE_FIELDS`/`PROVENANCE_FIELDS`/`OPTIONAL_EXTRAS`)
  into `test_conformance.py` rather than eliminating it — the same hidden-drift
  class the retirement is meant to end, now with no sync guard. Source the
  constants from a machine-readable artefact the Rust code owns, or add a
  drift-lockstep test.
  **Location**: Phase 5 (Change #4)

- 🔵 **Code Quality**: Naive flow-sequence parsing is duplicated a third time
  (`is_canonically_quoted` vs `linkage_elements` vs `parse_current_tags`).
  Extract one quote-aware splitter and reuse it (this is also the vehicle for
  the Critical fix).
  **Location**: Phase 3

- 🔵 **Code Quality**: The template-shape checks overload the instance-validation
  `Violation` enum with template-only variants. Give the template check its own
  `TemplateViolation` type.
  **Location**: Phase 5 (Change #1)

- 🔵 **Code Quality**: `m0008` "borrows m0007's private corpus-walk" — invites a
  copy-paste of `corpus_files`. State "extract and reuse" a shared enumeration
  helper.
  **Location**: Phase 2 (Change #2)

- 🔵 **Test Coverage**: Block-to-flow reflow of colon-bearing linkage sequences
  — the exact 0220 defect class — has no targeted test, only the aggregate
  self-check. Add a migration unit test reflowing a multi-element block linkage
  sequence to quoted flow and asserting it passes linkage-shape.
  **Location**: Phase 2 (Migration unit tests)

- 🔵 **Test Coverage**: Deleting `needs_quoting`/`format_tag` shifts special-char
  tag quoting onto the renderer with no end-to-end assertion that the persisted
  file quotes `"needs:colon"`. Add a `work update`/render test closing the loop.
  **Location**: Phase 1 (Changes #2, #3)

- 🔵 **Test Coverage**: The AC #6 discovery gate needs a two-tier allowlist —
  a single 21-entry list false-flags producer-marked but out-of-scope skills
  (`describe-pr`, `review-pr`, etc.), as the population test handles with a
  second tier.
  **Location**: Phase 4 — Static coverage test

- 🔵 **Test Coverage**: Validator edge cases untested — bare `type:` quoting
  (one of the most-migrated fields, currently bare `type: plan`) and the
  comma-in-quoted-flow-element symmetry. Add a bare/quoted `type` pair.
  **Location**: Phase 3 (Tests)

- 🔵 **Safety**: The preflight dirty-tree scan scopes a hardcoded set
  (`preflight.rs:16`) while `m0008`'s write scope is config-driven; a relocated
  corpus (via `paths.*` override) could be rewritten without the clean-tree
  guard scanning it, weakening revert-based recovery. Derive the scan from the
  actual write scope.
  **Location**: Migration Notes

- 🔵 **Compatibility**: The load-bearing block-fold suppression of `DoubleQuoted`
  on *long* scalars (the anti-churn benefit) is undocumented at the wrapper
  boundary and pinned by no test, on an exact-pinned pre-1.0 crate. Add an
  emitter test rendering a >80-column string and asserting single-line
  double-quoted output with no `>-`/`|`.
  **Location**: Phase 1 (Key Discoveries — `DoubleQuoted`)

#### Suggestions

- 🔵 **Correctness / Compatibility**: The float arm `DoubleQuoted(value.to_string())`
  renders `1.0` as `"1"` and round-trips a genuine float back as a String.
  Defensive-only today (no float value exists), but the same emitter governs
  plugin-user configs. Document the intended float→string coercion, or error at
  emit time; confirm 0227's config schema types no field as float.
  **Location**: Phase 1 (emitter arm)

- 🔵 **Code Quality**: The new validator helpers overlap existing ones
  (`is_bare_one` vs `is_bare_int`, `is_trailing_comment` vs
  `strip_trailing_comment`) and re-strip comments `is_quoted_scalar` already
  handles. Reuse the existing comment-handling path; keep one predicate per
  concept.
  **Location**: Phase 3 (Section 2)

- 🔵 **Code Quality**: `sync-work-items` and `implement-plan` gain a validate
  step only to satisfy the coverage test. Encode them as documented exceptions
  in the test, or confirm each re-validates a concrete file it touched.
  **Location**: Phase 4 (Section 1)

- 🔵 **Compatibility**: The work item claims `m0008` "realigns the sync
  baseline," but Phase 2 only re-renders `meta/`. Confirm whether a persisted
  sync baseline exists; if so, add its regeneration to the Phase 2 commit, else
  note the baseline is derived at sync time.
  **Location**: Phase 2 (Run and commit)

### Strengths

- ✅ The emitter is a true single choke point — one `Scalar::String` arm through
  which mapping values and (via `FlowSeq` recursion) sequence elements both
  pass, so the standard is set for all five write paths in one edit with minimal
  blast radius.
- ✅ The migration reuses the emitter as the single source of canonical
  *production* (`canonicalise_frontmatter` → `document::render`) instead of
  encoding a third quoting implementation — the right dependency direction,
  explicitly reasoned.
- ✅ Phase ordering is deliberate and correct (emitter → migrate → tighten
  validator → wire producers), keeping `this_repositorys_own_corpus_is_clean`
  green at every merge point.
- ✅ Strong TDD discipline: every phase has a red-first test section and every
  automated criterion states fails-before/passes-after, honouring the project's
  red-green-refactor non-negotiable; the proposed snippets are comment-free.
- ✅ The new `canonicalise_frontmatter` port defaults to `Err` (mirroring
  `validate_frontmatter`), failing closed; the Phase 5 deletion is gated behind
  its Rust replacement landing and being green.
- ✅ Scope boundaries are drawn accurately and defended: the visualiser is
  correctly identified as a byte-level `patch_status` mutation (not a renderer
  producer, verified to preserve quote style), and config validation is cleanly
  deferred to 0227.
- ✅ The `serde_saphyr` dependency risk is lower than feared — `DoubleQuoted` is
  a documented, crate-root-reexported public API, exact-pinned with a `deny.toml`
  upgrade-review boundary; `cli/work` public-api is correctly left un-regenerated
  because the tag helpers are private.

### Recommended Changes

1. **Make the Phase 3 predicate quote-aware** (addresses: the Correctness/
   Compatibility critical, the flow-parsing-duplication minor) — parse the
   closing quote first (reuse `is_quoted_scalar`'s existing approach), then look
   for a trailing comment only in the tail after it; make comma-splitting
   quote-aware via a single shared splitter. Add validator tests for a
   double-quoted value containing ` #` and a comma-bearing array element, and
   requote/verify the real `last_updated_note` file. This is the blocking fix.

2. **Guard the m0008 comment/CRLF loss for downstream corpora** (addresses: the
   Safety/Compatibility critical) — have `m0008` detect when re-rendering would
   drop a comment or change bytes beyond quoting/flow and surface a per-file
   diagnostic (follow `m0007`'s REFUSE/MALFORMED precedent), so the comment-free
   assumption is verified per run rather than assumed for every corpus. At
   minimum, scope AC #4's byte-identity guarantee explicitly to canonical/
   comment-free/LF input and document the comment-drop/CRLF-normalisation as an
   expected consequence.

3. **Add an emitter↔validator symmetry guard** (addresses: the Architecture/
   Code Quality major) — a paired round-trip/property test asserting the
   validator accepts every emitter output over a representative value-tree set,
   so future divergence fails a test rather than reaching a committed corpus.

4. **Fix the success-criteria commands** (addresses: the two Standards majors +
   the public-api minor) — replace `scripts:check` with
   `lint:skill-permissions:check`; drop the `corpus-adapters`/`corpus-cli`
   snapshot entries (only `cli/corpus` is pinned); replace `cli:check` for
   public-api work with `public-api:update`/`public-api:check`.

5. **Strengthen the enforcement proxies** (addresses: the AC #6 and idempotency
   Test Coverage majors + template-parity major) — bind AC #6 to the final
   section and a fenced block with a two-tier allowlist; add a real
   double-apply byte-equality idempotency test; require full per-check
   template-shape parity and gate the Python deletion on it.

6. **Close the remaining test gaps and code-hygiene items** (addresses: the
   minors/suggestions) — colon-bearing block-reflow test, special-char tag
   end-to-end test, bare-`type` pair, nested config round-trip; extract shared
   corpus-walk and flow-splitter helpers; give template-shape its own violation
   type; specify `is_bare_int`; decide the float-coercion contract.

---

## Per-Lens Results

### Architecture

**Summary**: Structurally sound plan built on a genuine single-point-of-change;
the emitter is one match arm through which every write funnels, and the
migration reuses it as the single source of canonical production. Phase
sequencing keeps the tree green, hexagonal discipline is preserved, and the
ADR's scoped-override draws a clean decision boundary. The principal risk is
evolutionary: "canonical" ends up encoded in three independent Rust surfaces
whose symmetry is a requirement bound by nothing structural, and Phase 5's
Python retirement relocates a cross-language constant mirror rather than
eliminating it.

**Strengths**:
- The emitter is a true single choke point (`Serialize for Yaml`, one
  `Scalar::String` arm; sequence elements recurse via `FlowSeq`).
- The migration reuses the emitter as the single source of canonical production
  rather than a third implementation.
- The Phase 5 template check respects the hexagonal boundary (pure core in
  `corpus`, FS driver in `corpus-adapters`, new `FrontmatterAction::ValidateTemplates`).
- Phase ordering keeps `this_repositorys_own_corpus_is_clean` green between
  merges.
- AC #6 coverage test is fail-closed (discovery gate over the live `skills/`
  tree).
- Scope boundaries accurate and defended (visualiser `patch_status`; config
  deferred to 0227).

**Findings**:
- 🟡 major (medium): Canonical predicate encoded in three independent surfaces
  with no binding symmetry guard (Implementation Approach; Phase 3; Phase 5).
  Emitter/validator split is unavoidable, but nothing binds them except a
  single-sample self-check; Phase 2 commits the corpus before Phase 3 tightens,
  so divergence surfaces only after commit, forcing re-migration. Add a
  property/round-trip symmetry test.
- 🔵 minor (medium): Retirement relocates a Rust→Python constant mirror into
  `test_conformance.py` rather than eliminating it (Phase 5 Change #4). The TSV
  is read dynamically but the three constant banks are hand-copied with no drift
  check. Source dynamically or add a lockstep guard.
- 🔵 minor (high): Config conformance rests solely on emitter correctness +
  idempotency, no independent checker until 0227 (Phase 2 config pass).
  Idempotency proves a fixed point, not canonicity; config's untyped nested tree
  is unseen by the flat corpus scanner. Strengthen the Phase 2 config test with
  a nested fixture and full canonical assertion.

### Correctness

**Summary**: Phasing, emitter reuse, and the retention of dedicated
id/schema_version/timestamp/linkage checks are sound, and migration idempotency
reasoning holds. But the proposed `is_canonically_quoted` has a boundary defect:
it strips trailing comments from the raw value before parsing the quote, so a
quoted scalar (or flow element) containing ` #` or an embedded comma is
truncated and mis-flagged `UNQUOTED-STRING` — actively breaking the self-check
gate on an existing corpus file and re-creating the emitter/validator
disagreement the work item exists to eliminate.

**Strengths**:
- Phase ordering correct: emitter and migration both produce output passing the
  old validator, so the tree stays green before Phase 3 tightens.
- Emitter-as-single-definition means migration correctness follows from
  emitter correctness.
- The predicate resists over-folding: `id`, `schema_version`, and linkage keep
  dedicated checks, preserving richer diagnostics.
- Idempotency well-founded: `render` has no omit-when-empty, corpus is
  comment/CRLF-free, parse→DoubleQuoted→emit is a genuine fixed point.

**Findings**:
- 🔴 critical (high): `is_canonically_quoted` strips trailing comments over the
  raw value before parsing the quote (Phase 3). Real file
  `2026-06-12-0108-local-docker-visual-regression-baselines.md:17`
  (`last_updated_note: "…question #1 — …"`) is truncated at ` #1`, loses its
  closing quote, and is mis-flagged `UNQUOTED-STRING` — turning
  `this_repositorys_own_corpus_is_clean` and the migration self-check RED. Parse
  the quote first, then look for a trailing comment only after the closing quote.
- 🟡 major (medium): The flow-collection branch splits on `,` and mis-splits a
  quoted element containing a comma (`["a, b"]`) or ` #` (`["needs#hash"]` — the
  value Phase 1's tag-rule deletion now produces). Reuse a quote-aware element
  splitter.
- 🔵 minor (medium): `is_bare_int` unspecified — a bare numeric string on a
  non-dedicated field (`external_id: 750`) passes as a bare integer, a false
  negative. Define grammar; note the limitation.
- 🔵 minor (medium): A bare malformed timestamp double-reports `UNQUOTED-STRING`
  + `BAD-TIMESTAMP`, contradicting the no-double-report claim. Accept/document or
  gate.
- 🔵 suggestion (low): `Scalar::Float` via `to_string()` renders `1.0` as `"1"`
  — a float→string round-trip change. Defensive-only today; assert/error on
  float at emit time or document the coercion.

### Code Quality

**Summary**: Well-structured and unusually disciplined on TDD (red-first tests
with explicit fails-before/passes-after) and comment hygiene (snippets are
comment-free). Its core instinct is sound: collapse quoting to one type-driven
choke point and let the migration re-render through it. The main risks are
duplicated encodings of the one "canonical" rule that can silently drift — a
hardcoded skip-list, naive flow parsing copied a third time, and the structural
emitter vs lexical validator — plus a cohesion question about overloading the
instance `Violation` enum.

**Strengths**:
- Strong TDD discipline across every phase.
- The emitter change is a genuine single-point edit with a small blast radius.
- The migration reuses the emitter rather than re-implementing the predicate.
- Clean separation of concerns in Phase 5 (pure `template_shape.rs` + FS driver).
- Proposed code is comment-free; phasing keeps the tree green.

**Findings**:
- 🟡 major (medium): `check_canonical_quoting` hardcodes a skip-list mirroring
  the dedicated checks (Phase 3) — implicit coupling; derive the skip-set from
  one source and test one-code-per-field.
- 🔵 minor (medium): Naive flow-sequence parsing duplicated a third time
  (`is_canonically_quoted` vs `linkage_elements` vs `parse_current_tags`).
  Extract one splitter.
- 🔵 minor (medium): Template-shape concerns overload the instance-validation
  `Violation` enum (Phase 5). Give it a `TemplateViolation` type.
- 🔵 minor (low): `m0008` borrows `m0007`'s private corpus-walk rather than
  reusing it (Phase 2). Extract and reuse a shared helper.
- 🔵 suggestion (medium): One canonical rule, two independent encodings that can
  drift (emitter vs validator); validator accepts `~` the emitter never emits.
  Add a paired round-trip test.
- 🔵 suggestion (low): New validator helpers overlap existing ones and re-strip
  comments already handled by `is_quoted_scalar`. Reuse the existing path.
- 🔵 suggestion (low): `sync-work-items` and `implement-plan` gain a validate
  step only to satisfy the coverage test. Encode as documented exceptions or
  confirm each re-validates a touched file.

### Test Coverage

**Summary**: Genuinely TDD-heavy with well-chosen integration gates
(`this_repositorys_own_corpus_is_clean`, byte-identity canary, public-api
snapshots, red-first AC #9). The most important risks are under-covered: the
render-idempotency / AC #4 byte-stability property is only on a synthetic
fixture (its real check is ledger-gated and ineffective); Phase 5 deletes a
~30-case Python suite while criteria require only "a broken template fails"; and
AC #6 — the sole enforcement — relies on a weak substring assertion.

**Strengths**:
- AC #9 emitter regression is red-first and drives both linkage and plain-string
  documents through the real render path.
- Phase ordering keeps the self-check gate green at every step.
- Producer write paths comprehensively inventoried (create/update/push +
  byte-identity canary).
- Public-api snapshots correctly treated as guard tests; AC #7 extends the
  existing exit-code contract.

**Findings**:
- 🟡 major (high): Render idempotency / AC #4 byte-stability lacks an effective
  automated test — the ledger-gated re-run is trivially true (Phase 2). Add a
  double-apply byte-equality test.
- 🟡 major (high): Phase 5 template-shape port risks silent coverage loss;
  criteria understate parity (~30 cases → one broken template). Require per-check
  parity; gate deletion on it.
- 🟡 major (medium): AC #6 uses a weak substring assertion for the sole
  enforcement mechanism (Phase 4). Bind to the final section + a fenced block.
- 🔵 minor (medium): AC #6 discovery gate needs a two-tier allowlist to avoid
  false-flagging out-of-scope producer-marked skills.
- 🔵 minor (medium): Block-to-flow reflow of colon-bearing linkage sequences —
  the 0220 defect class — is not covered by a targeted test.
- 🔵 minor (medium): Removing `needs_quoting`/`format_tag` shifts special-char
  tag quoting onto the renderer with no end-to-end test.
- 🔵 minor (low): Validator edge cases untested — bare `type:` quoting (a
  most-migrated field) and comma-in-quoted-flow-element symmetry.

### Safety

**Summary**: Reuses a proven, well-guarded migration framework (dirty-tree
refusal, atomic per-file writes, resumable manifest, in-process self-check,
VCS-revert recovery), fail-closes the new port, and gates the Python deletion
behind its Rust replacement. The principal gap: `m0008` is the first migration
to fully re-serialise frontmatter, silently dropping all inline comments and
normalising CRLF on every corpus it runs against — measured-safe for this repo
but unguarded for downstream corpora the shipped migration also rewrites — and
config.md's single write rests on stability-not-correctness.

**Strengths**:
- `canonicalise_frontmatter` defaults to `Err` (fails closed).
- Inherits the framework's proven safety envelope (preflight refusal, atomic
  writes, resumable manifest, VCS-revert recovery).
- Runs the in-process self-check; Phase 2 lands before Phase 3 tightens.
- The Python deletion is gated on the Rust replacement + re-sourced constants.
- The producer validate step is scoped to one `--file`.

**Findings**:
- 🔴 critical (high): `m0008` re-serialises and silently drops inline comments +
  normalises CRLF corpus-wide; the self-check is blind to it; ships to every
  downstream corpus via `accelerator migrate`. Detect and surface a per-file
  diagnostic (m0007 REFUSE/MALFORMED precedent).
- 🔵 minor (medium): config.md is re-rendered with no post-write validation
  (INVALID-TYPE); idempotency ≠ correctness, and it is the one operationally
  critical, unvalidated write. Add a round-trip semantic-equality check.
- 🔵 minor (low): The preflight dirty-tree scan is a hardcoded scope while
  `m0008`'s write scope is config-driven; a relocated corpus could be rewritten
  unscanned, weakening revert-based recovery. Derive the scan from the write
  scope.

### Standards

**Summary**: Strong fidelity to skill-authoring conventions — reuses the
plugin-root-prefixed allowed-tools format, preserves the per-file
two-vs-three-space indentation quirk, and its per-skill final-section headings
verify accurate. But it repeatedly misattributes Rust enforcement to the wrong
`mise` lanes: the SKILL.md permission lint is cited as `scripts:check` (shell)
when it is `lint:skill-permissions:check` (Python), and public-api work is
attributed to `cli:check`, which does not run cargo-public-api. It also invents
snapshots for two deliberately-exempt crate types.

**Strengths**:
- Correctly reuses the `${CLAUDE_PLUGIN_ROOT}`-prefixed `Bash(... *)` convention.
- The two-vs-three-space indentation claim verified accurate.
- Skill final-section headings verify accurate on every spot-check.
- Reuses `inventory-design`'s scrub-secrets gate as the model.
- New naming follows conventions (`UNQUOTED-STRING`, `UnquotedString`,
  `validate-templates`).
- Migration registration surface thoroughly enumerated; correctly recognises the
  sub-action rides the existing `corpus` sub-binary.

**Findings**:
- 🟡 major (high): SKILL.md tool-permission lint attributed to `scripts:check`;
  the actual guard is `lint:skill-permissions:check` (Phase 4).
- 🟡 major (high): Invents pinned public-api snapshots for exempt crate types —
  `corpus-adapters` (adapter) and `corpus-cli` (composition root) are not pinned
  (Phase 5 Change #5). Only `cli/corpus` needs regeneration.
- 🔵 minor (high): Public-api verification/regeneration attributed to
  `cli:check`, which does not run cargo-public-api; use
  `public-api:update`/`public-api:check` (Phases 1/3/5).
- 🔵 suggestion (medium): Re-sourcing schema constants into Python reintroduces
  a hand-synced duplication with no guard (Phase 5 Change #4). Prefer a
  machine-readable source or a lockstep guard.

### Compatibility

**Summary**: From a contract-stability standpoint the plan is well-grounded: the
quoting change is semantically transparent (quoted and bare parse identically,
so `work show`, PyYAML, and the visualiser `patch_status` — verified to preserve
existing quote style — keep working), the four public-api snapshots are correctly
scoped per phase (`cli/work` correctly left alone because the tag helpers are
private), and `validate-templates`/`UNQUOTED-STRING` are additive. The
`serde_saphyr` risk is lower than feared — `DoubleQuoted` is documented public
API, exact-pinned. The residual risks are two emitter/validator asymmetries: the
Phase 3 lexical check as sketched is not quote-aware, and `m0008` ships to every
plugin user's corpus where byte-identity degrades for commented/CRLF frontmatter.

**Strengths**:
- The migration is a byte-representation change only; all readers remain
  compatible; `patch_status` verified to preserve quote style.
- Public-api snapshots handled correctly and scoped per phase; `cli/work`
  correctly not regenerated (private helpers).
- New `validate-templates` action, `UnquotedString`, and the defaulted
  `canonicalise_frontmatter` port are additive.
- Producer allowed-tools rules require no bump to the v2.1.144 floor.
- Phase sequencing avoids any window where the corpus fails its own validator.

**Findings**:
- 🟡 major (medium): Phase 3's comment-strip and comma-split are not quote-aware,
  diverging from the emitter (Phase 3) — mis-flags quoted values containing ` #`
  or a comma; latent here (comment-free corpus), surfaces first in a plugin
  user's producer-run validation. Reuse the closing-quote-first approach.
- 🟡 major (medium): `m0008` ships to every plugin user and silently drops
  comments / normalises CRLF; byte-identity only holds for canonical input
  (Phase 2). Scope AC #4 explicitly and document the consequence.
- 🔵 minor (medium): The load-bearing block-fold suppression of `DoubleQuoted`
  on long scalars is undocumented at the wrapper boundary and pinned by no test,
  on a pre-1.0 exact-pinned crate. Add a long-value emitter test.
- 🔵 suggestion (low): Quoting floats changes a genuine float's round-trip type
  to String across plugin-user configs. Note as intended; confirm 0227 types no
  field as float.
- 🔵 suggestion (low): The "realigns the sync baseline" claim isn't realised by
  Phase 2's `meta/`-only re-render. Confirm whether a persisted baseline needs
  lockstep regeneration.

---

## Re-Review (Pass 2) — 2026-08-30

**Verdict:** REVISE

The pass-1 revision landed well: every pass-1 major is resolved and verified, and
both pass-1 criticals are fixed in direction. But the fresh reads found a **new
critical of the same class as pass 1** — the quote-aware predicate reuses the
existing `is_quoted_scalar`, which is itself blind to backslash-escaped inner
quotes, so a canonical `title: "… \"workflows\" …"` (two such files exist in the
committed corpus) is spuriously flagged, turning the Phase 3 self-check red on an
already-canonical file. The new mechanisms introduced in pass 1 (the `0008-LOSSY`
diagnostic, the shared helpers, the private predicate) also need tighter
specification, and the pass-1 sync-baseline suggestion escalated to a confirmed
corpus-scale interop hazard. Verdict stays REVISE: one new critical plus several
new majors, but the second wave is more tractable — specification gaps and a few
corrections to pass-1 edits, not fresh structural problems.

### Previously Identified Issues

- 🔴 **Correctness/Compatibility**: predicate not quote-aware (comment/comma) —
  **Partially resolved**. The ` #` and comma cases are fixed; the escaped-quote
  variant of the same class was newly found (see New Issues).
- 🔴 **Safety/Compatibility**: m0008 silent comment/CRLF loss — **Resolved in
  principle** (now surfaced via `0008-LOSSY`), but the surfacing mechanism has
  new gaps (fails open, under-specified detector — see New Issues).
- 🟡 **Architecture/Code Quality**: emitter↔validator symmetry guard —
  **Resolved** (guard added), but its value-tree set must include adversarial
  shapes (escaped quote, brackets, numeric-looking) to catch the very divergence
  it exists for.
- 🟡 **Test Coverage**: ineffective idempotency test — **Resolved** (double-apply
  byte-equality replaces the ledger-gated no-op).
- 🟡 **Test Coverage**: template-shape parity — **Resolved** (one negative per
  ported rule gates the Python deletion); a residual re-encoding concern remains
  (see New Issues).
- 🟡 **Test Coverage**: weak AC #6 substring — **Resolved** (section-bound,
  fenced-block, two-tier allowlist), but the heading regex does not match three
  in-scope skills' sections (see New Issues).
- 🟡 **Standards**: wrong `scripts:check` lane — **Resolved** (verified:
  `lint:skill-permissions:check` at `mise.toml:555`).
- 🟡 **Standards**: invented public-api snapshots — **Resolved** (verified: only
  `cli/corpus` pinned; `public-api:update`/`check` correct).
- 🟡 **Code Quality**: hardcoded skip-list — **Resolved** (`dedicated_check_keys`
  + one-code-per-field test); minor caveat that single-key checks needn't iterate
  it.

### New Issues Introduced

- 🔴 **Correctness (confirmed, high)**: `is_quoted_scalar`'s `rest.find('"')`
  stops at an escaped inner `"`, so a canonical string with an embedded escaped
  quote is mis-flagged `UNQUOTED-STRING`. Real files:
  `2026-06-29-0176-workflows-rename-and-skill-catalogue.md:4` and
  `2026-08-07-0172-migration-engine-subdomain.md`. Make the closing-quote scan
  (and the flow splitter) find the first *unescaped* `"`; add the case to the
  quote-aware tests and the symmetry guard.
- 🟡 **Architecture/Code Quality**: the canonical-quoting predicate is private in
  `cli/corpus` instance-validation, but Phase 5 templates and work item 0227
  config both need it — and templates hand-rewritten to canonical quoting are not
  actually validated for it. Extract to a shared pure module; have
  `validate-templates` run `check_canonical_quoting`; reuse existing pure
  predicates (`is_quoted_scalar`, `is_bare_one`) rather than re-encoding.
- 🟡 **Safety/Architecture (high)**: the preflight fix ("derive the scan from
  m0008's write scope") is wrong — preflight runs once per run for *all*
  migrations, and the exposure predates m0008 (`m0006` already walks
  `paths.plans`). Fix at the run-level `SCOPES` (config-path-aware for every
  migration) and promote it from a prose caveat to an owned Phase 2 deliverable +
  test.
- 🟡 **Compatibility (confirmed, high)**: corpus re-quoting invalidates every
  `last-sync.json` `local_hash` (`digest::local` hashes raw frontmatter lines
  including quotes; `IGNORE_KEYS` excludes none of the re-quoted fields), so the
  next `/sync-work-items` classifies every item as locally-modified and floods
  the tracker. Downstream users are uncovered. Promote to a concrete task: m0008
  recomputes each baseline `local_hash`, preserves `remote_hash`, diagnoses, and
  is regression-tested.
- 🟡 **Safety**: `0008-LOSSY` fails open (exit 0, "applied") with per-file lines
  scrolling by in a 1000+ file walk and no aggregate summary. Keep proceed, but
  emit an end-of-run summary ("N files lost comments/CRLF — revert to recover")
  so the loss cannot be missed.
- 🟡 **Safety/Code Quality**: the `0008-LOSSY` detector is described two ways
  ("change bytes beyond quoting/flow" vs "drops comments/CRLF") and under-tested.
  Fix to the tractable form (inline `#` outside quotes, or CRLF, or non-UTF-8
  round-trip failure) and test all quadrants including a clean-file negative.
- 🟡 **Code Quality/Correctness**: do not share `linkage_elements`' splitter — it
  is deliberately non-quote-aware (`split('#')`, splits every comma) and feeds two
  tested consumers. Extract only the innermost quote-aware comma split; keep
  `linkage_elements`' bracket/comment wrapper and empty-element filter (or
  `[]`/`["needs#hash"]` break).
- 🟡 **Code Quality**: the Python-constant re-sourcing needs a concrete decision —
  the "machine-readable artefact the Rust owns" does not exist for the three
  banks. Emit a Rust-owned data file or add a `--print-schema` subcommand + guard,
  not a regex over `.rs` source.
- 🟡 **Architecture/Code Quality**: the shared corpus-walk must take the dir set
  as a parameter — `m0007`'s `corpus_files` is linkage-filtered, so sharing it
  couples m0008 completeness to the linkage vocabulary. m0008 supplies
  `doc_type_dirs()`; test a doc type lacking a linkage type name.
- 🟡 **Test Coverage**: `_HEADING_RE` does not match `## Capturing Changes`
  (stress-test skills) or `## Verification Approach` (implement-plan), so the
  section-bound AC #6 assertion misfires on three in-scope skills. Extend the
  regex or scope the coverage test's own heading set.
- 🔵 **minor** (Correctness/CQ/TC/Safety/Compat): config round-trip must
  hard-fail (abort), distinct from `0008-LOSSY`; Phase 3 references a nonexistent
  test file (`cli/corpus/tests/frontmatter.rs` → validator tests live in
  `mod.rs`) and under-enumerates fixtures (the inline `--checks structure` case);
  the special-char tag test must drop the comma sub-case (naive resplit); `rfind(']')`
  mislocates the closing bracket on a trailing comment containing `]`;
  `is_bare_int` accepts a bare zero-padded value on non-`id` fields; the float
  coercion contract with 0227 is prose-only; apply the value-tree round-trip guard
  to `meta/` files, not only config; add a direct `config-adapters` store-write
  test for AC #3.
- 🔵 **suggestion** (Standards/Compat/Arch/CQ): note the `cli/work` snapshot is
  unaffected (private helpers); record `UnquotedString` in the violation taxonomy
  header; record the invocation-string convention in one canonical place; consider
  `TagMutation::Changed(Vec<String>)` to drop the lossy re-parse; extract the
  coverage-test heading/discovery helpers to a shared non-test support module.

### Assessment

The plan is materially stronger and its architecture is sound, but it is not yet
ready to implement: one confirmed new critical (escaped-quote mis-flagging) blocks
it, and several majors — the sync-baseline flood and the run-level preflight fix
in particular — are load-bearing for correctness and downstream safety and are
currently under-specified or mis-scoped. The remaining work is a focused second
iteration: harden the quote-aware predicate for escapes, extract the shared
predicate, tighten the `0008-LOSSY` and config-round-trip mechanisms, and turn the
sync-baseline and preflight items into owned deliverables with tests.

---

## Re-Review (Pass 3) — 2026-08-30

**Verdict:** REVISE

Focused pass over the four lenses that carried pass-2's critical and
high-confidence majors (Correctness, Safety, Compatibility, Architecture). The
escaped-quote critical is **resolved** (escape-aware `closing_quote`, verified
correct on `\"`/`\\\"`/lone-trailing-`\\` without panic), and every pass-2 major
is closed and verified. The wave surfaced five new majors — none critical, all
fine-grained corrections to the pass-2 edits: two crate-boundary violations (the
sync-baseline realignment cannot live in `cli/migrate`, and the shared predicate's
home in `cli/corpus` does not reach the config domain), and two edge-case blind
spots in the new safety nets (value-preservation is blind to parse-time
normalisation; the baseline realignment masks genuine unpushed edits). These are
the increasingly fine points expected as a plan converges; the structure is sound
and the remaining items are bounded.

### Previously Identified Issues (Pass 2)

- 🔴 escaped-quote mis-flagging — **Resolved**. `closing_quote`/`closing_bracket`
  find the first unescaped `"`; the two real corpus files and adversarial inputs
  are in the tests and symmetry guard.
- 🟡 predicate placement / templates-not-validated — **Resolved in direction**
  (extracted to a shared module; templates now run the check), but the module's
  home does not reach the config domain (new major below).
- 🟡 preflight run-level fix — **Resolved in direction** (change #7), but the
  scope union omits `paths.integrations`/`paths.templates` (new major below).
- 🟡 sync-baseline flood — **Resolved in direction** (change #6), but the change
  as sited breaches crate discipline and masks genuine edits (new majors below).
- 🟡 `0008-LOSSY` fails open / under-specified — **Resolved** (tractable detector,
  aggregate summary, sensitivity+specificity tests).
- 🟡 config round-trip fail-closed; shared corpus-walk param'd; Python constants;
  `_HEADING_RE`; flow-splitter — **Resolved and verified**.

### New Issues Introduced

- 🟡 **Architecture (high)**: change #6 sites the baseline realignment in
  `cli/migrate/m0008.rs`, but `digest::local` lives in `work-adapters` and the
  `migrate` crate may depend only on corpus/document/kernel (enforced by the
  `migrate_domain_imports_only_permitted` pup rule). Route it through a new
  `MigrationContext` port implemented in `migrate-adapters`; acknowledge/justify
  the `migrate-adapters → work-adapters` edge, or relocate the shared digest.
- 🟡 **Compatibility (medium)**: change #6 recomputes `local_hash` for *every*
  entry, so an item genuinely edited-and-committed-but-unpushed before the
  migration is reclassified Synced and its pending push is silently lost. Realign
  only entries whose pre-migration `digest::local` equalled the stored baseline;
  add a test that a genuinely-modified item stays flagged across the migration.
- 🟡 **Architecture (medium)**: the shared `canonical_quoting` module homed in
  `cli/corpus` does not reach 0227's config validator without a new config→corpus
  domain edge (both depend only on kernel). Place the pure predicate where both
  domains already reach — kernel, document, or a small dedicated crate.
- 🟡 **Correctness (medium)**: the value-preservation check compares the
  re-parsed tree to the *pre-render parse*, so it is blind to parse-time
  normalisation — a bare `id: 0042` deserialises to `Int(42)`, renders bare `42`,
  and re-parses to `Int(42)`, matching while silently stripping zero-padding. It
  catches only render-introduced drift (the float case), not parse-introduced.
  Gate each re-render on the in-process validator passing, and add a raw-scalar
  comparison for bare values the standard requires quoted.
- 🟡 **Safety (medium)**: change #7's preflight union covers doc-type/corpus dirs
  but not `paths.integrations` (where change #6 writes) or `paths.templates`
  (m0006). Define the union as every config-driven path any registered migration
  writes; extend the relocated-corpus test to a relocated integrations dir.
- 🔵 **minor**: `m0008`'s behaviour when the in-process `validate_frontmatter`
  reports a residual violation mid-walk is undefined — specify fail-closed abort;
  the validator flow predicate handles only one `[...]` level while the emitter
  recurses (add a nested-sequence case to the symmetry guard or make the predicate
  recurse); the parse/render-failure abort should name the file and verify
  in-memory before writing; the `0008-LOSSY` aggregate count should reach the
  migrate skill's own output, not only engine stderr.
- 🔵 **suggestion**: prefer a build-regenerated static schema data file over the
  `print-schema` subcommand (or mark it internal and pin its JSON shape); add a
  renderer-level guard/test that no field serialises as a float, enforcing the
  0227 no-float precondition in code; resolve the template-shape reuse seam
  (refactor shared `check_*` to return field lists, or reuse only leaf predicates).

### Assessment

The plan is close. One class of remaining major is crate-discipline placement
(two ports/homes to relocate), the other is edge-case completeness in the two new
safety nets (both one-directional as written). None is a structural flaw and none
is critical. A final focused iteration on these five majors — the two port/home
relocations, the two realignment/value-net edge cases, and the preflight union —
would clear the path to implementation; the minors are well-suited to being caught
during TDD. The re-review loop is converging: pass 1 found structural gaps, pass 2
found specification gaps in the fixes, pass 3 finds boundary and placement details.

### Resolution (applied after Pass 3, by author direction)

All five Pass-3 majors were applied to the plan; the re-review loop was then
concluded (the remaining Pass-3 minors and suggestions are deferred to TDD during
implementation, not re-reviewed):

- Sync-baseline realignment routed through a new `realign_sync_baseline`
  `MigrationContext` port implemented in `migrate-adapters` (keeps `migrate`
  domain-pure), and scoped to realign only pre-migration-`Synced` entries so a
  pending unpushed edit is not masked (Phase 2 change #6).
- Value-preservation extended with a raw-scalar guard for parse-time
  normalisation (the padded-`id` class) and an in-process `validate_frontmatter`
  fail-closed gate, checked in memory before writing (Phase 2 change #2).
- Preflight union broadened to every config-driven migration write path,
  including `paths.integrations` and `paths.templates`, with tests for both a
  relocated corpus and a relocated baseline (Phase 2 change #7).
- Shared `canonical_quoting` predicate re-homed to a location both the corpus and
  config domains reach without a `config → corpus` edge (kernel/document/dedicated
  crate), so 0227 can reuse it (Phase 3).

These edits were not themselves re-reviewed; the loop was stopped here by author
direction on the basis that the plan is thoroughly specified and the residual
items are TDD-appropriate.

### Final verdict: APPROVE — 2026-08-30

Approved for implementation by author direction after the Pass-3 majors were
applied. Every critical and major raised across the three passes is addressed in
the plan; the remaining Pass-3 minors and suggestions are accepted as
TDD-appropriate and are not blockers. The frontmatter `verdict` reflects this
final APPROVE; the Pass-1/2/3 sections above are preserved as the review history
and their in-body verdicts record each pass's state at the time.

---
*Review generated by /accelerator:review-plan*
