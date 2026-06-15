---
type: plan
id: "2026-06-15-0105-close-corpus-validator-provenance-and-linkage-blind-spots"
title: "Close the Corpus Validator Provenance and Linkage Blind Spots Implementation Plan"
date: "2026-06-15T19:21:05+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0105"
parent: "work-item:0105"
derived_from: ["codebase-research:2026-06-15-0105-corpus-validator-provenance-linkage-blind-spots"]
tags: [frontmatter, schema, validator, provenance, linkage]
revision: "9b3d121e111f9f676a8e27762efa42af640bed52"
repository: "build-system"
last_updated: "2026-06-15T21:21:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Close the Corpus Validator Provenance and Linkage Blind Spots Implementation Plan

## Overview

Fold two known blind spots into `scripts/validate-corpus-frontmatter.sh` so the
single corpus-frontmatter oracle enforces them directly:

1. **Provenance over-emission** — reject `revision`/`repository` on a
   *non-anchored* type (the provenance "iff" is currently enforced only in the
   forward direction).
2. **Bare/unquoted typed-linkage values** — reject a present-but-non-empty
   linkage value that is not a quoted `"doc-type:id"` reference (the shape loop
   only ever inspects *quoted* tokens, so a bare `parent: 0042` escapes).

Once both rules are live, **delete** the two bespoke helpers in
`scripts/test-skill-frontmatter-conformance.sh`
(`check_no_provenance_over_emission`, `check_linkage_quoted`) that currently
re-derive these rules by inspection, returning the contract to a single
authority.

## Current State Analysis

The validator is a bash-3.2-safe, single-helper (`violation()`) checker driven
by a violation counter. Per-type facts (`code_state_anchored`,
`typed_linkage_keys`, …) load from `templates-schema.tsv` into parallel arrays;
cross-cutting sets (`FM_PROVENANCE_FIELDS`, `FM_TYPED_REF_RE`, …) are sourced
from `frontmatter-emission-rules.sh`. Both blind spots live in `validate_file`.

**Blind spot 1 — provenance, `validate-corpus-frontmatter.sh:295-305`:**

```bash
  # Provenance bundle iff code_state_anchored=yes; git_commit/branch never.
  if [ "$anchored" = "yes" ]; then
    for f in "${FM_PROVENANCE_FIELDS[@]}"; do
      bk_present "$f" ||
        violation "$file" "MISSING-PROVENANCE" "anchored type missing provenance field '$f'"
    done
  fi
  for f in "${FM_FORBIDDEN_PROVENANCE_FIELDS[@]}"; do
    bk_present "$f" &&
      violation "$file" "FORBIDDEN-PROVENANCE" "legacy provenance field '$f' present"
  done
```

Only the forward direction (`anchored=yes ⇒ present`) is enforced. A
non-anchored type carrying `revision`/`repository` is silently accepted. The
`:295` comment overstates what is enforced.

**Blind spot 2 — linkage, `validate-corpus-frontmatter.sh:334-357`:**

```bash
  # Typed-linkage values: doc-type:id shape + (corpus mode) referential.
  local key rest tok
  for key in $linkkeys; do
    bk_value "$key" || continue
    rest="$BK_VAL"
    while [[ "$rest" =~ \"([^\"]*)\" ]]; do
      tok="${BASH_REMATCH[1]}"
      rest="${rest#*\""${tok}"\"}"
      [ -n "$tok" ] || continue
      if [[ ! "$tok" =~ $FM_TYPED_REF_RE ]]; then
        violation "$file" "BAD-LINKAGE-SHAPE" "$key: '$tok' is not a typed \"doc-type:id\" reference"
        continue
      fi
      if [ "$referential" = "yes" ]; then
        case "$tok" in
          pr:*) : ;; # tolerated external-entity prefix
          *)
            index_has "$tok" ||
              violation "$file" "DANGLING-REF" "$key: '$tok' resolves to no artifact in the corpus"
            ;;
        esac
      fi
    done
  done
```

The `while` only matches `"…"` tokens. A bare scalar (`parent: 0042`), a bare
path (`parent: meta/work/0030-foo.md`), or a bracketed-but-unquoted element
(`parent: [plan:0042]`) produces **zero** tokens, so the body never runs and
`BAD-LINKAGE-SHAPE` never fires. A *quoted* bad value (`parent: "0042"`,
`parent: "meta/work/0030-foo.md"`) does produce a token and is already caught by
the shape regex — which is why the existing "bare-number"/"path-shape" fixtures
(both *quoted*) pass today and the genuinely-unquoted case is the real gap.

**Three-authority temporary state (the thing being unwound):**
`scripts/test-skill-frontmatter-conformance.sh` carries two bespoke helpers that
re-derive these rules by inspecting templates/skills directly (BYPASSING the
validator), tagged `[0105]`:

- `check_no_provenance_over_emission` (`:180-188`) + per-emitter call site
  (`:314-316`) + liveness self-tests (`:408-414`).
- `check_linkage_quoted` (`:190-209`) + per-emitter call site (`:317-320`) +
  liveness self-tests (`:417-422`).
- Banner naming 0105 at `:176-177`; header-comment item 4 at `:16-18`.

### Key Discoveries:

- **No contract-file edits needed.** The new rules *read* `FM_PROVENANCE_FIELDS`
  and `FM_TYPED_REF_RE` (`frontmatter-emission-rules.sh:34,88`); the new
  diagnostic `PROVENANCE-ON-NONANCHORED` is a literal in the validator,
  not a contract constant. Neither `templates-schema.tsv` nor
  `frontmatter-emission-rules.sh` is modified — so the 0104 merge-ordering
  concern (those two files) does **not** apply to 0105's actual edits.
- **`assert_rejects` already asserts a specific code** (`frontmatter-fixtures.sh:75-88`,
  `grep -qF -- "$code"` + non-zero rc), so AC3 ("specific diagnostic, not merely
  non-zero exit") needs no new tooling.
- **The legacy-forbid loop (`:302-305`) is the template** for the provenance
  rule: an unconditional `bk_present && violation` over a field array. The new
  rule is the same shape, gated on `anchored != yes`.
- **Reuse `BAD-LINKAGE-SHAPE`** for the unquoted sub-case (resolved in the work
  item's Open Questions — same violation class). This keeps the tamper guard at
  `test-validate-corpus-frontmatter.sh:163` (the only literal
  `grep -qF "BAD-LINKAGE-SHAPE"`) untouched.
- **Real-corpus sanity check** (`test-validate-corpus-frontmatter.sh:180-191`)
  validates the live `meta/` corpus clean — a strong regression backstop for the
  linkage-tokenizer rewrite. All existing linkage fixtures (quoted scalar,
  quoted path, dotted-stem, note-source, bracketed list) must also stay green.
- **bash 3.2 floor.** No associative arrays, no `${var,,}`. The whitespace-trim
  idiom `${v#"${v%%[![:space:]]*}"}` and `case`-glob quoting checks already used
  in this file are the safe building blocks.

## Desired End State

`validate-corpus-frontmatter.sh` rejects (a) any non-anchored type carrying a
`FM_PROVENANCE_FIELDS` member with `PROVENANCE-ON-NONANCHORED`, and (b)
any present-but-non-empty typed-linkage value that is not a quoted
`"doc-type:id"` reference with `BAD-LINKAGE-SHAPE` — including genuinely
unquoted scalars, unquoted paths, bracketed-but-unquoted elements, and mixed
lists with one bad element. `test-validate-corpus-frontmatter.sh` carries
failure-mode fixtures for both, each asserting the specific diagnostic. The two
bespoke helpers in `test-skill-frontmatter-conformance.sh` are gone, their
intent folded into that guard's conditional-axis coverage, run through the real
validator. `mise run test:integration:config` and `mise run test:unit:templates`
are green.

Verify: `mise run test:integration:config` (runs both `test-*` suites) exits 0;
the new fixtures fail if reverted; `grep -rn '\[0105\]' scripts/` returns
nothing.

## What We're NOT Doing

- **No cardinality enforcement.** The shape loop will keep ignoring
  `fm_linkage_cardinality` (single vs list); 0105 does not require a
  `single`-key-carries-no-list rule. (Research Open Questions; out of scope.)
- **No new diagnostic code for linkage.** Reuse `BAD-LINKAGE-SHAPE`; do not
  introduce `BARE-LINKAGE-VALUE` (resolved).
- **No edits to `templates-schema.tsv` or `frontmatter-emission-rules.sh`** —
  the rules stay data-driven readers.
- **No changes to the validator's other rules** (base fields, id quoting,
  timestamps, status vocab, omit-when-empty, referential integrity).
- **No "reduce to liveness" half-measure for Phase 3** — the helpers are
  deleted outright (decided).
- **No flagging of an embedded empty element inside a non-empty list.** A value
  like `relates_to: ["plan:0001", ""]` is neither the literal `[]`/`""` that
  `EMPTY-PLACEHOLDER` matches (the omit-when-empty loop checks the whole value)
  nor caught by the tokenizer's empty-`inner` `continue`, so it passes silently.
  This is a pre-existing narrow gap the rewrite neither closes nor widens;
  uniform embedded-empty handling is out of scope for 0105.

## Implementation Approach

Three phases. Phases 1 and 2 are mutually independent (different code blocks in
the validator, different fixtures) and each mergeable on its own. Phase 3
depends on **both** 1 and 2 (it deletes helpers covering both axes and adds
conditional-axis reject cases that only pass once both validator rules exist).
TDD within each phase: write the failing fixture(s) first, then the rule, then
confirm the whole suite + real-corpus sanity stays green.

---

## Phase 1: Provenance over-emission rule

### Overview

Reject `revision`/`repository` on a non-anchored type via a complementary
`else` branch, emitting `PROVENANCE-ON-NONANCHORED`. Correct the `:295`
comment to state the now-bidirectional iff.

### Changes Required:

#### 1. Failing fixtures first (TDD)

**File**: `scripts/test-validate-corpus-frontmatter.sh`
**Changes**: In the failure-mode section (after the `FORBIDDEN-PROVENANCE`
fixture at `:64-65`), add a non-anchored type carrying the provenance bundle.
`emit_valid` only adds provenance for anchored types, so inject it via
`extra_lines`:

```bash
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-prov-nonanchored.md" \
  $'revision: "abc123"\nrepository: "repo"'
assert_rejects "non-anchored type with provenance rejected" \
  "PROVENANCE-ON-NONANCHORED" "$TMP/bad-prov-nonanchored.md"

# Single-field variant (only revision) still trips the rule.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-prov-revision-only.md" \
  'revision: "abc123"'
assert_rejects "non-anchored type with lone revision rejected" \
  "PROVENANCE-ON-NONANCHORED" "$TMP/bad-prov-revision-only.md"
```

Run `bash scripts/test-validate-corpus-frontmatter.sh` → these two FAIL (rule
not yet present). The existing anchored-type valid fixtures already cover the
accept side (anchored `plan`/`note`/etc. carry provenance and pass).

#### 2. The rule

**File**: `scripts/validate-corpus-frontmatter.sh` (`:295-305`)
**Changes**: Convert the forward-only `if` into `if/else`; correct the comment.

```bash
  # Provenance bundle iff code_state_anchored=yes (both directions enforced);
  # git_commit/branch never.
  if [ "$anchored" = "yes" ]; then
    for f in "${FM_PROVENANCE_FIELDS[@]}"; do
      bk_present "$f" ||
        violation "$file" "MISSING-PROVENANCE" "anchored type missing provenance field '$f'"
    done
  else
    for f in "${FM_PROVENANCE_FIELDS[@]}"; do
      bk_present "$f" &&
        violation "$file" "PROVENANCE-ON-NONANCHORED" "non-anchored type carries provenance field '$f'"
    done
  fi
  for f in "${FM_FORBIDDEN_PROVENANCE_FIELDS[@]}"; do
    bk_present "$f" &&
      violation "$file" "FORBIDDEN-PROVENANCE" "legacy provenance field '$f' present"
  done
```

The `:295` comment is tightened to state the now-bidirectional iff explicitly
(`(both directions enforced)`) rather than leaving the bare "iff" that
historically overstated what the forward-only rule enforced. The long
`violation` message lines match the file's existing convention — the sibling
`MISSING-PROVENANCE`/`BAD-LINKAGE-SHAPE`/`DANGLING-REF` message lines already
run 92–101 columns and pass `scripts:check` (shfmt does not wrap or flag
string-argument lines; the 80-col `.editorconfig` rule is not gated for shell).

### Success Criteria:

#### Automated Verification:

- [x] Both new fixtures pass: `bash scripts/test-validate-corpus-frontmatter.sh`
- [x] Full config suite green: `mise run test:integration:config`
- [x] Shell lint/format clean: `mise run scripts:check`
- [x] Real-corpus sanity still clean (covered by the suite's section 5)

#### Manual Verification:

- [x] Hand-run on a crafted non-anchored fixture with `revision:` prints
      `PROVENANCE-ON-NONANCHORED` and exits non-zero.
- [x] An anchored type (e.g. `plan`) still requires provenance (no regression to
      `MISSING-PROVENANCE`).

---

## Phase 2: Bare/unquoted typed-linkage rule

### Overview

Rewrite the linkage tokenizer so every non-empty element of a linkage value is
asserted to be a quoted `"doc-type:id"` token before the shape regex. Bare
scalars, bare paths, bracketed-but-unquoted elements, and mixed lists with a bad
element are rejected with `BAD-LINKAGE-SHAPE`. All currently-accepted/rejected
shapes are preserved.

### Changes Required:

#### 1. Failing fixtures first (TDD)

**File**: `scripts/test-validate-corpus-frontmatter.sh`
**Changes**: After the existing *quoted*-malformed linkage fixtures (`:73-77`),
add genuinely-*unquoted* cases (distinct from `parent: "0030"` which the loop
already catches):

```bash
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-unquoted-linkage.md" 'parent: 0030'
assert_rejects "unquoted (bare) linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-unquoted-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-unquoted-path-linkage.md" 'parent: meta/work/0030-foo.md'
assert_rejects "unquoted path linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-unquoted-path-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-bracket-unquoted-linkage.md" 'parent: [plan:0042]'
assert_rejects "bracketed-but-unquoted linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-bracket-unquoted-linkage.md"

emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-mixed-list-linkage.md" 'relates_to: ["plan:0001", plan:0002]'
assert_rejects "mixed list with one unquoted element rejected" "BAD-LINKAGE-SHAPE" "$TMP/bad-mixed-list-linkage.md"
```

Also add **accept-side** fixtures that guard the rewrite's new comma-split path
directly (the only existing bracketed-list accept fixture, `ok-dotted-linkage`,
is single-element, so multi-element splitting is otherwise unverified):

```bash
# Well-formed MULTI-element quoted list still accepts (exercises comma-split).
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-multi-list-linkage.md" 'relates_to: ["adr:0001", "adr:0002"]'
assert_accepts "multi-element quoted list accepted" "$TMP/ok-multi-list-linkage.md"

# Irregular inter-element spacing still accepts (per-element trim).
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-spaced-list-linkage.md" 'relates_to: ["adr:0001",   "adr:0002"]'
assert_accepts "irregularly-spaced quoted list accepted" "$TMP/ok-spaced-list-linkage.md"

# Trailing inline comment on a quoted scalar still accepts (comment strip).
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/ok-comment-linkage.md" 'parent: "work-item:0001" # inverse note'
assert_accepts "quoted ref with trailing inline comment accepted" "$TMP/ok-comment-linkage.md"

# set -f glob suppression: an UNQUOTED glob-bearing value must reject with the
# LITERAL token, regardless of CWD contents — proving the comma-split does not
# pathname-expand. Run from a directory seeded with files that WOULD match.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-glob-linkage.md" 'parent: plan-*'
mkdir -p "$TMP/globdir"
: >"$TMP/globdir/plan-1.md"
: >"$TMP/globdir/plan-2.md"
glob_rc=0
glob_err="$(cd "$TMP/globdir" && "$VALIDATOR" "$TMP/bad-glob-linkage.md" 2>&1 >/dev/null)" || glob_rc=$?
if [ "$glob_rc" -ne 0 ] &&
  grep -qF -- "BAD-LINKAGE-SHAPE" <<<"$glob_err" &&
  grep -qF -- "plan-*" <<<"$glob_err"; then
  echo "  PASS: unquoted glob value rejects with literal token (globbing suppressed)"
  PASS=$((PASS + 1))
else
  echo "  FAIL: glob-bearing linkage not deterministic (rc=$glob_rc): $glob_err"
  FAIL=$((FAIL + 1))
fi
```

And pin the **no-double-flag** invariant the rewrite relies on: an empty
linkage value must yield `EMPTY-PLACEHOLDER` *only*, never an additional
`BAD-LINKAGE-SHAPE`. The existing `bad-empty.md` fixture (`:70-71`) greps for
one code and would pass even if a spurious second diagnostic appeared, so assert
the absence explicitly. Add a small reusable negative-assertion helper alongside
`assert_rejects`/`assert_accepts` in `scripts/frontmatter-fixtures.sh` (the
shared helper file both suites source) so the intent matches the suite's
vocabulary rather than hand-rolling counter bookkeeping inline:

```bash
# In scripts/frontmatter-fixtures.sh, beside assert_rejects.
# rc-agnostic by design: pairs with an assert_rejects that pins the expected
# diagnostic + non-zero rc; assert_absent only verifies a spurious second code
# is NOT also emitted.
assert_absent() { # $1=name $2=code; remaining args = validator args
  local name="$1" code="$2"
  shift 2
  run_validator "$@"
  if grep -qF -- "$code" <<<"$VALIDATOR_ERR"; then
    echo "  FAIL: $name (unexpected code '$code' present)"
    # shellcheck disable=SC2001 # anchored whole-line sed indent that ${var//.../...} cannot express
    echo "$VALIDATOR_ERR" | sed 's/^/    /'
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: $name"
    PASS=$((PASS + 1))
  fi
}
```

Cover BOTH empty forms — the quoted-empty `parent: ""` (caught by the inner
`[ -n "$inner" ]` skip) and the bracketed-empty `relates_to: []` (caught by the
post-bracket-strip empty `$rest`, a *different* tokenizer branch):

```bash
# `parent: ""` must NOT also emit BAD-LINKAGE-SHAPE (reuses the :70 fixture).
assert_absent "empty quoted linkage does not double-flag" "BAD-LINKAGE-SHAPE" "$TMP/bad-empty.md"

# `relates_to: []` (bracketed-empty) likewise emits EMPTY-PLACEHOLDER only.
emit_valid work-item no "$BASE_EXTRAS" "$BASE_VOCAB" "$TMP/bad-empty-list.md" 'relates_to: []'
assert_rejects "empty-list placeholder rejected" "EMPTY-PLACEHOLDER" "$TMP/bad-empty-list.md"
assert_absent "empty-list linkage does not double-flag" "BAD-LINKAGE-SHAPE" "$TMP/bad-empty-list.md"
```

Run the suite → the four reject fixtures FAIL (loop sees zero/partial tokens
today); the accept and no-double-flag assertions already pass and lock the
behaviour in. The existing accept fixtures (`ok-dotted-linkage`,
`ok-note-source`) and quoted-bad fixtures (`bad-bare-linkage`, `bad-path-linkage`)
must continue to pass after the rewrite — together with the new multi-element
and trailing-comment fixtures they are the regression guard for the tokenizer.

#### 2. The rewritten tokenizer

**File**: `scripts/validate-corpus-frontmatter.sh` (`:334-357`)
**Changes**: Replace the quoted-substring `while` with: strip a trailing inline
comment, strip a surrounding `[…]`, comma-split (with globbing disabled), and
assert each non-empty element is a quoted token.

```bash
  # Typed-linkage values: each non-empty element must be a quoted "doc-type:id"
  # reference (bare/unquoted and path-shaped values rejected) + (corpus mode)
  # referential integrity. The strip/split below rely on the FM_TYPED_REF_RE id
  # grammar ([A-Za-z0-9.-]+): refs contain no '#', '[', ']', or ',', so the
  # comment/bracket strips and comma-split are lossless for well-formed values.
  local key rest elem inner oldifs
  for key in $linkkeys; do
    bk_value "$key" || continue
    rest="$BK_VAL"
    rest="${rest%%#*}"                         # strip trailing YAML inline comment (refs contain no '#')
    rest="${rest%"${rest##*[![:space:]]}"}"    # re-trim trailing whitespace
    rest="${rest#\[}"                          # strip an optional surrounding flow-list bracket
    rest="${rest%\]}"
    oldifs="$IFS"
    IFS=','
    set -f                                     # comma-split only — suppress pathname (glob) expansion
    for elem in $rest; do
      elem="${elem#"${elem%%[![:space:]]*}"}"  # trim leading whitespace
      elem="${elem%"${elem##*[![:space:]]}"}"  # trim trailing whitespace
      [ -n "$elem" ] || continue               # empty element ([] / trailing comma)
      case "$elem" in
        '"'*'"')
          inner="${elem#\"}"                   # same-quote-pair strip (case proved both ends are ")
          inner="${inner%\"}"
          [ -n "$inner" ] || continue          # empty quoted ("") — handled by EMPTY-PLACEHOLDER
          if [[ ! "$inner" =~ $FM_TYPED_REF_RE ]]; then
            violation "$file" "BAD-LINKAGE-SHAPE" "$key: '$inner' is not a well-formed \"doc-type:id\" reference"
            continue
          fi
          if [ "$referential" = "yes" ]; then
            case "$inner" in
              pr:*) : ;; # tolerated external-entity prefix
              *)
                index_has "$inner" ||
                  violation "$file" "DANGLING-REF" "$key: '$inner' resolves to no artifact in the corpus"
                ;;
            esac
          fi
          ;;
        *)
          violation "$file" "BAD-LINKAGE-SHAPE" "$key: unquoted value '$elem' is not a well-formed \"doc-type:id\" reference"
          ;;
      esac
    done
    IFS="$oldifs"
    set +f                                     # restore default globbing (script never runs noglob globally)
  done
```

Rationale for each guard:
- Comment strip + trailing-ws re-trim preserves today's tolerance of a trailing
  inline comment. The accurate invariant is that no value byte *before* the
  comment is `#` — guaranteed by the `FM_TYPED_REF_RE` id grammar
  (`[A-Za-z0-9.-]+`), cross-referenced in the block comment so a future charset
  widening that admits `#`/`[`/`]`/`,` is a visible coupling, not silent
  corruption.
- `set -f` / `set +f` brackets the comma-split so the unquoted `$rest`
  expansion word-splits on `,` *without* pathname (glob) expansion — otherwise a
  value with a surviving glob metacharacter would expand against the validator's
  CWD, producing cwd-dependent, non-deterministic diagnostics. The script never
  enables `noglob` globally, so the unconditional `set +f` restore is correct.
- The two `BAD-LINKAGE-SHAPE` arms share the tail `is not a well-formed
  "doc-type:id" reference`; the unquoted arm prefixes `unquoted value` so the
  variant reads as a qualifier on one message (single code, specifics in the
  message — per the work item's resolved Open Question), and no consumer greps
  the message text (the tamper guard at `:163` matches the code literal only).
- Bracket strip makes scalars and flow-lists tokenize identically; typed-ref ids
  cannot contain `[`/`]`/`,`, so the strip-then-split is lossless.
- `[ -n "$inner" ] || continue` preserves today's behaviour for `parent: ""`
  (skipped here; flagged once by `EMPTY-PLACEHOLDER`, no double-violation).
- The `*)` arm is the blind-spot fix: a bare element reaches it and emits
  `BAD-LINKAGE-SHAPE` with the unquoted-distinguishing message.
- `IFS` is set only inside the per-key body and restored after; the outer
  `for key in $linkkeys` pre-splits with default IFS at entry.

### Success Criteria:

#### Automated Verification:

- [x] All four new reject fixtures pass; the new accept fixtures
      (multi-element list, irregular spacing, trailing inline comment) and the
      no-double-flag assertion pass; all existing linkage fixtures still pass:
      `bash scripts/test-validate-corpus-frontmatter.sh`
- [x] Tamper guard (`:163`) still green (work-item dropped from vocab still
      yields `BAD-LINKAGE-SHAPE` on the quoted ref).
- [x] `set -f` glob suppression holds: the `bad-glob-linkage` fixture (Section 1)
      — an unquoted `parent: plan-*` validated from a directory seeded with
      `plan-1.md`/`plan-2.md` — rejects with `BAD-LINKAGE-SHAPE` naming the
      literal `plan-*`, not the expanded filenames. This fixture goes red if the
      `set -f`/`set +f` bracket is dropped, so the glob-suppression guarantee is
      mutation-proof rather than asserted only in prose. (A *quoted* `"plan:*"`
      would be caught by the shape regex regardless of `set -f`, so the unquoted
      value reaching the `*)` arm is what actually exercises the glob path.)
- [x] Real-corpus sanity clean (section 5) — no regression on live artifacts.
- [x] Full config suite green: `mise run test:integration:config`
- [x] Shell lint/format clean: `mise run scripts:check`

#### Manual Verification:

- [x] Hand-run on `parent: 0042`, `parent: meta/work/x.md`, `parent: [plan:0042]`,
      and `relates_to: ["plan:0001", plan:0002]` each prints `BAD-LINKAGE-SHAPE`
      and exits non-zero.
- [x] Hand-run on a well-formed quoted list (`relates_to: ["adr:0001", "adr:0002"]`)
      exits 0.

---

## Phase 3: Delete the bespoke conformance-guard helpers

### Overview

With both rules live in the validator, delete `check_no_provenance_over_emission`
and `check_linkage_quoted` (and all their wiring) from
`test-skill-frontmatter-conformance.sh`, folding their intent into the existing
conditional-axis coverage so it exercises the validator's new rules. Depends on
Phases 1 **and** 2.

**Template-source inspection of these two axes is intentionally dropped — an
accepted tradeoff, not a hidden one.** The deleted helpers were the *only* place
that grepped on-disk `templates/*.md` and skill substitute-lists directly for
over-emitted provenance / bare-linkage slots. Nothing in the surviving guard
replaces that at the template-*source* level:

- The per-`(skill, type)` composed-acceptance loop (`:306-310`) does **not**
  cover it. `emit_valid` (`frontmatter-fixtures.sh:31-66`) synthesises a minimal
  artifact from the schema row — it never reads template content, drives
  provenance solely from the schema `anchored` flag, and omits all typed-linkage
  keys — so a template hard-coding `revision:` on a non-anchored type or a bare
  `parent: 0042` produces an identical clean fixture and still passes. The
  composed-*completeness* check (`:288-304`) only inspects template key *names*
  for presence, never forbidden values.
- The section-5 real-corpus sanity check (`test-validate-corpus-frontmatter.sh:180-191`)
  walks live `meta/` **artifacts**, not `templates/`. It is a downstream
  regression backstop — it catches an already-emitted defective artifact (for
  artifact types that exist in `meta/`), not the template defect itself.

This coverage reduction is the deliberate consequence of "delete the helpers
outright" (work item 0105 Open Question, resolved). The justification: templates
are author-controlled, reviewed source, and any defect they introduce surfaces
the moment a producer emits an artifact the validator then rejects. If a
template-source guard is wanted later, it should be a *new* validator-routed lint
over `templates/*.md` frontmatter — out of scope for 0105, but worth a follow-up
work item. The plan must not claim this coverage is "preserved."

### Changes Required:

#### 1. Remove the helpers and their wiring

**File**: `scripts/test-skill-frontmatter-conformance.sh`
**Changes**:
- Delete `check_no_provenance_over_emission` (`:180-188`) and
  `check_linkage_quoted` (`:190-209`), plus the `[0105]` banner comment block at
  `:176-177` ("Blind-spot checks (BYPASS the validator …)").
- Delete the two per-emitter `assert_check … [0105]` call sites (`:312-320`).
- Delete the "Blind-spot liveness" self-test block (`:404-422`).
- Keep `assert_check` (`:212-224`) — still used by the `status_in_vocab`
  assertions (`:286,:333,:345`).
- Update the file-header docblock: drop item 4 (`:16-18`, the blind-spot bypass
  description) since the validator now enforces both axes.

#### 2. Fold intent into the conditional-axis section

**File**: `scripts/test-skill-frontmatter-conformance.sh` (`:351-377`)
**Changes**: This section already exercises provenance/linkage through the
**real** validator. Strengthen it so the previously-uncaught bad cases are now
asserted-rejected:

- Switch the bare-linkage case (`:368-369`) from the *quoted* `parent: "0042"`
  to a genuinely *unquoted* value, so it exercises the new rule:

```bash
emit_valid work-item no "kind priority external_id" "draft" "$TMP/link-bare.md" 'parent: 0042'
assert_rejects "bare (unquoted) linkage rejected" "BAD-LINKAGE-SHAPE" "$TMP/link-bare.md"
```

- Add a non-anchored-provenance reject beside the provenance present/absent/
  missing cases (`:355-361`):

```bash
emit_valid work-item no "kind priority external_id" "draft" "$TMP/prov-overemit.md" \
  $'revision: "x"\nrepository: "y"'
assert_rejects "provenance on non-anchored type rejected" \
  "PROVENANCE-ON-NONANCHORED" "$TMP/prov-overemit.md"
```

These run only green once Phases 1+2 have landed, which is the intended
dependency.

- **Keep the accept-side controls.** The deleted liveness block (`:404-422`)
  carried clean controls (anchored-provenance-allowed `:413-414`,
  quoted-linkage-accepted `:421-422`) proving the checks did not over-fire. The
  conditional-axis section already keeps the validator-routed equivalents —
  `prov-present` (`:356`), `prov-absent` (`:358`), `link-present` (`:365`),
  `link-absent` (`:367`) — so an over-eager new rule (wrongly rejecting an
  anchored type's provenance, or a valid quoted ref) is still caught. Leave
  these four assertions in place; they are the accept-side replacement for the
  deleted controls. (Note `prov-absent` is precisely the non-anchored-with-no-
  provenance accept case that pairs with the new `prov-overemit` reject.)

- **The negative-wiring proof comes from the `emit_valid`-built reject fixtures,
  not a sed mutation.** The deleted liveness self-tests proved each bespoke check
  *could* fail. With the helpers gone and the rules in the validator, the
  automated equivalent is the two reject fixtures above (`prov-overemit`,
  `link-bare`): both are synthesised by `emit_valid` with genuinely-bad content
  (extra provenance lines / an unquoted `parent`), so each is definitionally a
  non-clean fixture (no "is the mutation a no-op?" guard is needed), and each
  goes from rejecting to **accepting** the moment its validator rule is reverted
  — which is exactly the "this assertion can fail" property the liveness block
  provided, now routed through the real oracle.

  **Do NOT extend the `assert_axis_mutation` block (`:380-402`) for these axes.**
  That helper mutates an existing line via `sed`; provenance/linkage have no
  line in the clean `$BASE` to mutate, so injecting one would require a newline
  in the `sed` replacement (`s/.../tags: []\nrevision: "x".../`). BSD/macOS sed
  (the bash-3.2 floor) treats `\n` in the replacement as a literal `n`, not a
  newline, producing a single mangled line — a macOS-only failure. Building the
  bad fixture with `emit_valid` (C-style `$'…\n…'`, which bash expands before
  `printf`) sidesteps the BSD-sed pitfall entirely, which is why the reject
  fixtures above are the right vehicle.

#### 3. Confirm no orphaned references

**Changes**: `grep -rn '\[0105\]' scripts/` returns nothing;
`grep -rn 'check_no_provenance_over_emission\|check_linkage_quoted' scripts/`
returns nothing. The "No re-encoded contract" meta-asserts (`:426-429`) still
pass (the guard still sources `frontmatter-emission-rules.sh` and reads the TSV).

### Success Criteria:

#### Automated Verification:

- [ ] Guard suite green: `bash scripts/test-skill-frontmatter-conformance.sh`
- [ ] No orphaned 0105/helper references: `grep -rn '\[0105\]' scripts/` and
      `grep -rn 'check_no_provenance_over_emission\|check_linkage_quoted' scripts/`
      both empty.
- [ ] Producer-set reconciliation + count assertions still pass (16 emitters
      processed; discovery returns 17).
- [ ] Full config suite green: `mise run test:integration:config`
- [ ] `mise run test:unit:templates` green.
- [ ] Shell lint/format clean: `mise run scripts:check`

#### Manual Verification:

- [ ] Reverting either validator rule turns the conformance guard's new
      conditional-axis reject case red (proves the guard is tied to the rule, not
      green-path-only).

---

## Testing Strategy

### Unit / suite tests:

- **`test-validate-corpus-frontmatter.sh`** — the validator's behaviour suite;
  gains six fixtures (two provenance, four linkage). `assert_rejects` pins the
  exact diagnostic. Existing accept/reject fixtures + the tamper guard + the
  real-corpus sanity check are the regression guard for the tokenizer rewrite.
- **`test-skill-frontmatter-conformance.sh`** — the producer-conformance guard;
  loses the two bespoke helpers and their self-tests, gains two
  validator-routed conditional-axis assertions.

### Integration tests:

- `mise run test:integration:config` glob-runs **both** `test-*.sh` suites
  (`tasks/test/helpers.py:13-35`), so the new fixtures execute automatically —
  tying suite greenness to the new rules firing (work item AC5).
- `mise run test:unit:templates` runs `test-template-frontmatter.sh` (the other
  contract-sourcing surface) — must stay green (no contract-file edits, so it
  should be untouched).

### Edge cases to verify manually:

1. `parent: 0042` (bare scalar) → `BAD-LINKAGE-SHAPE`.
2. `parent: meta/work/0030-foo.md` (bare path) → `BAD-LINKAGE-SHAPE`.
3. `parent: [plan:0042]` (bracketed unquoted) → `BAD-LINKAGE-SHAPE`.
4. `relates_to: ["plan:0001", plan:0002]` (mixed) → `BAD-LINKAGE-SHAPE`.
5. `parent: "work-item:0042"` and `relates_to: ["adr:0001","adr:0002"]` → accept.
6. `parent: ""` / `relates_to: []` → `EMPTY-PLACEHOLDER` only (no double-flag,
   no `BAD-LINKAGE-SHAPE`).
7. Non-anchored `work-item` with `revision:`/`repository:` →
   `PROVENANCE-ON-NONANCHORED`; anchored `plan` without them →
   `MISSING-PROVENANCE` (unchanged).

## Performance Considerations

Negligible. Both rules are O(fields) per file over the same arrays already
iterated; the linkage rewrite replaces one regex `while` with a comma-split
`for` over the same short value string. The whole-corpus walk's watchdog budget
is unaffected.

## Migration Notes

None. No data migration; no contract-file change. The real corpus already
conforms (sanity check proves it), so no existing artifact is newly rejected.

## References

- Original work item: `meta/work/0105-close-corpus-validator-provenance-and-linkage-blind-spots.md`
- Research: `meta/research/codebase/2026-06-15-0105-corpus-validator-provenance-linkage-blind-spots.md`
- Provenance block: `scripts/validate-corpus-frontmatter.sh:295-305`
- Linkage loop: `scripts/validate-corpus-frontmatter.sh:334-357`
- Contract constants: `scripts/frontmatter-emission-rules.sh:34` (`FM_PROVENANCE_FIELDS`),
  `:88` (`FM_TYPED_REF_RE`)
- Fixture helpers: `scripts/frontmatter-fixtures.sh:31-103`
- Validator suite fixtures: `scripts/test-validate-corpus-frontmatter.sh:64-81,163`
- Bespoke helpers to delete: `scripts/test-skill-frontmatter-conformance.sh:176-209,312-320,404-422`
- Task wiring: `tasks/test/integration.py:46-64`, `tasks/test/helpers.py:13-35`,
  `tasks/test/unit.py:34-41`
