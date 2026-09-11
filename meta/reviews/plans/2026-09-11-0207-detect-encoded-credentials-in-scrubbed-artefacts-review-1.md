---
type: "plan-review"
id: "2026-09-11-0207-detect-encoded-credentials-in-scrubbed-artefacts-review-1"
title: "Plan Review: Detect Encoded Credentials In Scrubbed Artefacts Implementation Plan"
date: "2026-09-10T23:27:29+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "plan:2026-09-11-0207-detect-encoded-credentials-in-scrubbed-artefacts"
target: "plan:2026-09-11-0207-detect-encoded-credentials-in-scrubbed-artefacts"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "security", "safety", "test-coverage", "code-quality", "compatibility"]
review_number: 1
review_pass: 5
tags: ["security", "secrets", "design"]
last_updated: "2026-09-11T14:13:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Detect Encoded Credentials In Scrubbed Artefacts Implementation Plan

**Verdict:** REVISE

The plan is architecturally sound and lands cleanly on the module's existing
needle-vs-haystack seams, decomposing the work into pure, char-based,
independently testable functions with well-bounded phases; the name-only
disclosure invariant and the exit-1/no-artefact guarantee are preserved across
every new form, and all three dependency claims verify against the real tree.
Two critical issues block it: Phase 3 derives a head-prefix from the *whole*
`AUTH_HEADER` value, manufacturing the needle `Authorizatio`, which breaks the
existing `the_header_name_alone_does_not_false_positive` test — so Phase 3 is
not `mise run` green as claimed — and the plan's own headline safety property
(char-vs-byte slicing) is asserted but has no multi-byte UTF-8 test. A cluster
of major encoding-coverage gaps (base64url/no-pad, lowercase-hex and partial
percent-encoding, hex) are neither derived nor listed in scope.

### Cross-Cutting Themes

- **Phase 3 head-prefix over-breadth** (flagged by: safety, correctness) — the
  12-char head is derived from every form including low-entropy raw values, so
  `Authorization: Bearer …` yields `Authorizatio` and `https://example.com/…`
  yields `https://exam`. The first is a substring of ordinary prose about an
  "Authorization header" and breaks an existing false-positive test; the second
  refuses any artefact citing the same host. Because refusal is unrecoverable
  (no `--force`), each false positive permanently blocks legitimate content.
- **Encoding coverage is narrower than the threat, and the gap is undocumented**
  (flagged by: security, correctness) — only STANDARD base64 and maximal
  RFC-3986 percent-encoding are derived. base64url, no-padding base64,
  lowercase-hex percent (`%2f` vs `%2F`), partial percent-encoding (reserved
  delimiters left literal), and hex-encoded material all evade, and none appear
  in "What We're NOT Doing". For a last-barrier secret guard, each residual form
  is a real leak path that should be either derived or named as a reviewed
  exclusion.
- **The headline safety invariant is asserted but untested** (flagged by:
  test-coverage; the char-based choice is praised by correctness, safety,
  code-quality) — Key Discoveries pins char-vs-byte slicing as load-bearing to
  avoid a panic, yet every proposed test value is ASCII. A regression to
  byte-indexing would panic at runtime with the suite green.
- **Doc and comment drift** (flagged by: code-quality, compatibility, security)
  — the module doc-comment reword is specified only in the abstract, the
  non-obvious value-half rationale doc-comment is silently dropped, and the
  shipped `docs-site/.../design.md` still describes "literal value" matching.

### Tradeoff Analysis

- **False-negative reduction vs. false-positive / availability cost**: security
  pushes to widen what is caught (more encodings); safety warns that broader
  matching plus an unrecoverable refusal converts any false positive into a hard
  block on legitimate developer content. The head-prefix is where this bites
  hardest. Recommendation: restrict head-prefix derivation to high-entropy forms
  (encoded forms and the header value-half, never the whole `Name: value`
  string or raw structural URLs), and treat encoding breadth as a *documented*
  best-effort — derive the high-value variants (base64url, lowercase-hex
  percent) and explicitly enumerate the rest in "What We're NOT Doing" rather
  than chasing every form.

### Findings

#### Critical

- 🔴 **Safety + Correctness**: Head-prefix of the whole `AUTH_HEADER` value is
  the header name, reintroducing a false positive that fails an existing test
  **Location**: Phase 3: Minimum-length head-prefix matching, `prefixes()`
  Phase 3 derives a 12-char head from every form including the raw whole value
  `Authorization: Bearer abc123` (28 chars, ≥16), giving the needle
  `Authorizatio` — a substring of "Authorization" in ordinary prose. The
  existing `the_header_name_alone_does_not_false_positive`
  (`leaked_credentials.rs:98-109`) asserts such prose must not flag, so it fails
  under Phase 3, contradicting "each phase leaves `mise run` green". The same
  root cause makes any `LOGIN_URL` produce a needle like `https://exam` that
  refuses unrelated documents citing the host. Fix: derive head-prefixes only
  from secret-bearing forms (the header value-half and raw non-header values, or
  the encoded forms), never from the whole `Name: value` string; add a
  regression test pinning the header-name case.

- 🔴 **Test Coverage**: The flagged multi-byte UTF-8 panic risk has no test
  **Location**: Phase 3, Section 2: Tests / Key Discoveries
  Key Discoveries flags char-vs-byte slicing as the load-bearing property
  (`chars().count() >= 16` / `chars().take(12)`, never `f.len()` / `f[..12]`) to
  avoid a non-char-boundary panic, yet every proposed value is ASCII. A
  regression to byte-based length or slicing would panic in a security-critical
  scanner with the whole suite green. Add a domain-unit test using a ≥16-char
  value with multi-byte codepoints straddling the 12th-character boundary,
  asserting the prefix is derived and matched without panic.

#### Major

- 🟡 **Security + Correctness**: base64url and no-padding base64 variants slip
  past silently
  **Location**: Phase 1 #2 (`encodings()`); What We're NOT Doing
  Only STANDARD base64 (`+`/`/`, padded) is derived. URL-safe (`-`/`_`) and
  no-pad forms are common for tokens and URL-embedded secrets — exactly the
  `AUTH_HEADER` / JWT world — and match neither the STANDARD needle nor the
  whitespace-strip haystack. Either derive `URL_SAFE`/`URL_SAFE_NO_PAD` (and
  standard no-pad), or name them in "What We're NOT Doing" with a rationale.

- 🟡 **Security + Correctness**: percent-encoding covers only maximal,
  uppercase-hex form
  **Location**: Phase 1 #2 (`UNRESERVED_ESCAPES` / `percent_encode`)
  Two distinct gaps compound. (1) `percent_encode` emits uppercase hex, but
  `%2f` and `%2F` are RFC-3986-equivalent and models/browsers frequently emit
  lowercase, so `a%2fb` evades while `a%2Fb` is caught. (2) The needle escapes
  everything but the unreserved set; real encoders commonly leave reserved
  delimiters literal (`a%20b/c`), so any partially-encoded rendering between
  verbatim and maximal matches neither derived form. Derive both hex casings
  (and consider a reserved-set-only encoding), or document the boundary.

- 🟡 **Security**: Hex encoding is entirely absent and unlisted
  **Location**: What We're NOT Doing; Desired End State
  Hex is one of the most common renderings of token/binary credential material,
  yet it is neither derived nor mentioned. Lowercase, uppercase, and mixed-case
  hex all slip. Either derive a hex form (deciding case deliberately) or list it
  in "What We're NOT Doing".

- 🟡 **Test Coverage**: The ≥16 minimum-form threshold is not pinned at its
  exact 15/16 edge
  **Location**: Phase 3, Section 2 (`a_form_shorter_than_sixteen_gains_no_prefix`)
  The 12-char prefix boundary is pinned both sides, but `MINIMUM_FORM_LENGTH =
  16` is not: the short-value test uses a 4-char head, far below the edge. A
  mutation of `>= 16` to `>= 15`, `> 16`, or `>= 14` fails no test. Add a
  boundary pair: exactly 15 → no prefix needle, exactly 16 → prefix.

#### Minor

- 🔵 **Safety**: Broadened matching has no per-form diagnostic — a false refusal
  shows only "the value of {name} appears … in some form", making a broad-prefix
  false positive slow to distinguish from a true leak. Consider surfacing the
  matched form-class (literal / encoded / reflowed / head-prefix) without
  emitting the value.
  **Location**: What We're NOT Doing (unrecoverable refusal) / Overview

- 🔵 **Security**: Residual leak surface under-enumerated and doc-comment risks
  overclaiming — "What We're NOT Doing" lists only case folding, form-encoding,
  and mid/tail truncation; the reworded module doc-comment must stay precise
  about exactly which encodings are derived so a maintainer does not assume
  coverage that is absent.
  **Location**: Phase 1 #2 (module doc-comment, `leaked_credentials.rs:6-9`)

- 🔵 **Security**: HTML-entity and JSON `\u` escape forms unaddressed in an
  HTML/JSON-heavy domain — a credential inside an HTML- or JSON-escaped snippet
  evades and is not listed as out of scope.
  **Location**: What We're NOT Doing

- 🔵 **Correctness**: Values 12–15 characters long have an undocumented
  head-truncation gap — a form in that band gains no prefix and is matched whole
  only, while a 16+ form is prefixed. Acceptable if deliberate; state it beside
  the mid/tail-truncation exclusion.
  **Location**: Phase 3, `prefixes()` threshold

- 🔵 **Test Coverage**: Domain tests do not pair name-present with value-absent
  as the strategy claims — the shown Phase 1 tests assert only `scan(...) ==
  vec!["NAME"]`, and the name-only invariant is reasserted once. Align the
  wording or the tests.
  **Location**: Testing Strategy: Unit Tests / Phase 1 Section 5

- 🔵 **Test Coverage**: Compound shapes only partially pinned — reflow+base64 is
  covered, but reflow+percent, reflow+prefix, and prefix-of-header-value-half
  are not. Add at least the leading-12 prefix of an encoded header value-half.
  **Location**: Phase 2 & Phase 3, Section 2: Tests

- 🔵 **Code Quality**: Module doc-comment update specified only in the abstract
  — "stop disclaiming encoding derivation" gives no replacement text for the
  module's load-bearing intent statement. Specify the exact prose.
  **Location**: Phase 1, Section 2

- 🔵 **Code Quality**: The header value-half rationale doc-comment is silently
  dropped — the non-obvious *why* of the first-colon split
  (`leaked_credentials.rs:19-24`) has no home once the logic moves to
  `values()`. CLAUDE.md says to keep exactly this kind of comment; relocate it
  onto `values()`.
  **Location**: Phase 1 Section 2 / Phase 3 Section 1

- 🔵 **Compatibility**: Shipped docs not updated —
  `docs-site/src/content/docs/design.md:146` still states scrub-secrets refuses
  "the literal value", now inaccurate. Add the docs reword to Phase 1; note
  `docs:check` is outside the aggregate `check` so verify it directly.
  **Location**: Phase 1, Section 4

#### Suggestions

- 🔵 **Code Quality**: Helper names misdescribe their returns — `encodings()`
  returns `[value, base64, percent]` (forms, incl. identity) and `prefixes()`
  returns `[form, head]`. Consider `forms()` and `form_and_head()`.
  **Location**: Phase 1 / Phase 3 helper definitions

- 🔵 **Code Quality**: `UNRESERVED_ESCAPES` names the complement of what it holds
  — it is the set that *gets* escaped (everything but unreserved). Rename to
  `PERCENT_ESCAPE_SET` or similar.
  **Location**: Phase 1, Section 2

- 🔵 **Code Quality**: Homogeneous haystack array forces a needless full-body
  clone — `[body.to_owned(), strip_whitespace(body)]` clones the raw body only
  to unify the array type. Bind `let stripped = …;` and test `body.contains(n)
  || stripped.contains(n)`.
  **Location**: Phase 2, Section 1

- 🔵 **Security**: Double / nested encoding not considered — low likelihood in
  prose; add a one-line exclusion so the single-level scope is deliberate.
  **Location**: What We're NOT Doing

- 🔵 **Architecture**: Domain crate gains its first external deps — admitting
  `base64`/`percent-encoding` into the previously kernel-only core is a
  conscious boundary decision; state it so future additions get the same
  scrutiny.
  **Location**: Phase 1, Section 1

- 🔵 **Architecture**: Evasion-defeating logic is split across two seams (needle
  transforms vs. haystack transform) — keep the mermaid pipeline as the anchor
  and add a one-line invariant to the module doc ("needles widen *what*;
  haystack transforms widen *where*").
  **Location**: Key Discoveries / Phase 2, Section 1

- 🔵 **Architecture**: The "no artefact written" guarantee lives in an external
  caller (`main.rs:93-104` + invoking hook/skill), not the scanner — since 0207
  broadens detection, pin the caller-side contract (exit 1 ⇒ artefact must not
  be promoted) explicitly in the plan.
  **Location**: Key Discoveries / Desired End State

- 🔵 **Architecture**: The first-colon header split is a contract shared with the
  browser daemon with no shared abstraction — pre-existing and correctly scoped
  out, but record the coupling so a future daemon change keeps the scanner in
  lockstep.
  **Location**: Current State Analysis / Phase 1 Section 2

### Strengths

- ✅ Extends the existing needle-vs-haystack seam rather than bolting on a
  parallel path; new logic stays in the pure functional core with no I/O added
  and `scan()`'s signature unchanged.
- ✅ Derivation decomposes into small, pure, independently unit-testable free
  functions (`encodings`, `prefixes`, `strip_whitespace`, `values`) with
  domain-aligned naming.
- ✅ The name-only disclosure invariant holds across every new form: `scan()`
  maps to `secret.name.clone()` only, and the reject-message reword still
  interpolates `{name}` alone — no security guarantee weakened.
- ✅ Char-based length and slicing correctly designed to avoid a UTF-8 boundary
  panic; `PREFIX_LENGTH = 12` is a clean base64 block-boundary multiple.
- ✅ The empty-value guard and secret-order output contract are preserved ahead
  of all new needle forms.
- ✅ Out-of-scope decisions are pinned as negative tests (mid/tail truncation,
  high-entropy substring), and the 12-char prefix boundary is pinned both sides.
- ✅ The `env_clear()` isolation claim holds — `subcommands.rs run()` clears the
  environment, so a developer's own `ACCELERATOR_BROWSER_*` cannot leak in.
- ✅ All three dependency claims verify: `base64` 0.22.1 and `percent-encoding`
  2.3.2 each appear exactly once in `cli/Cargo.lock`, both dual MIT/Apache-2.0
  in `deny.toml`'s allow-list, so caret bounds add no package and cannot trip
  `multiple-versions = "warn"`; private helpers leave `cargo-public-api` untouched.

### Recommended Changes

1. **Restrict head-prefix derivation to secret-bearing / high-entropy forms**
   (addresses: the two 🔴/🟡 prefix findings) — in Phase 3, do not run
   `prefixes()` over the whole `Name: value` header string or raw structural
   values; derive head-prefixes from the header value-half and the encoded forms
   only (or exclude the header-name portion before prefixing). Add regression
   tests: prose containing "Authorization" does not flag; a `LOGIN_URL`-host
   prefix in an unrelated document does not flag. Re-verify
   `the_header_name_alone_does_not_false_positive` stays green.

2. **Add a multi-byte UTF-8 prefix test** (addresses: the 🔴 test-coverage
   finding) — a ≥16-char value with multi-byte codepoints straddling the 12th
   character, asserting the leading-12 prefix is derived and matched without
   panic.

3. **Close or document the encoding gaps** (addresses: base64 variants,
   percent-encoding coverage, hex) — derive the high-value forms
   (`URL_SAFE`/`URL_SAFE_NO_PAD`, lowercase-hex percent) with tests, and
   explicitly enumerate the deliberately-excluded families (partial
   percent-encoding, hex if not derived, HTML/JSON escapes, double-encoding) in
   "What We're NOT Doing". Keep the reworded module doc-comment precise about
   exactly which forms are derived.

4. **Pin the ≥16 threshold at its 15/16 edge** (addresses: the 🟡 test-coverage
   finding) — boundary pair mirroring the existing 12-char two-sided style.

5. **Specify and complete the doc changes** (addresses: doc-drift minors) —
   give the exact replacement module doc-comment prose; relocate the value-half
   rationale onto `values()`; add the `docs-site/.../design.md:146` reword to
   Phase 1's change list.

6. **Apply the naming and allocation cleanups** (addresses: code-quality
   suggestions) — `encodings`→`forms`, `prefixes`→`form_and_head`,
   `UNRESERVED_ESCAPES`→`PERCENT_ESCAPE_SET`; drop the full-body clone in
   `scan()` for `body.contains(n) || stripped.contains(n)`.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound — extends the two boundaries the module
already draws (needles = what to search, scan = where), keeping all new logic in
the pure functional core with no I/O. The composable `flat_map` pipeline (values
→ encodings → prefixes) is open for extension without modifying the matcher, and
the key tradeoffs are explicitly acknowledged. Observations concern the domain
crate's first coupling to third-party encoding libraries, the matching-shape
concern now split across two seams, and the pre-existing split of the security
guarantee across a process boundary.

**Strengths**:
- Preserves and reinforces the two-seam separation, each seam with a single
  reason to change.
- The needle pipeline is a composable, open-for-extension `flat_map` chain; a
  future encoding is additive to `encodings()`, not a `scan()` rewrite.
- Keeps the domain crate a pure functional core; every new function is pure and
  unit-testable, `scan()` signature unchanged.
- Domain-aligned naming (`values`, `encodings`, `prefixes`, `strip_whitespace`).
- Tradeoffs surfaced and bounded rather than hidden.

**Findings**:
- 🔵 minor (high): Domain crate gains its first external deps (base64,
  percent-encoding) — a deliberate coupling of the core to third-party
  libraries; state it as a conscious boundary decision. (Phase 1, Section 1)
- 🔵 minor (medium): Evasion-defeating logic split across two seams (needle
  transforms vs. haystack transform) — keep the mermaid diagram as anchor and
  add an invariant note to the module doc. (Key Discoveries / Phase 2, Section 1)
- 🔵 minor (medium): The security guarantee is enforced by an external caller
  (`main.rs:93-104` + hook/skill), not the scanner — pin the caller-side
  contract in the plan since 0207 broadens detection. (Key Discoveries / Desired
  End State)
- 🔵 suggestion (low): The first-colon header split is a contract shared with the
  browser daemon with no shared abstraction — record the coupling. (Current
  State Analysis / Phase 1 Section 2)

### Correctness

**Summary**: Logically careful in most respects — char-based length/slicing
avoids UTF-8 panics, the empty-value guard and secret-order output are
preserved, and the base64 prefix length (12, a multiple of 4) lands on a clean
block boundary. But Phase 3's head-prefix is applied to the whole `AUTH_HEADER`
value, whose 12-char head `Authorizatio` is the header-name prefix, reintroducing
a false positive an existing test pins — so `mise run` would fail. Secondary gaps
in percent-encoding hex casing and base64 alphabet variants.

**Strengths**:
- Mandates `chars().count()` / `chars().take(12)` over byte indexing.
- `PREFIX_LENGTH` 12 lands on a clean base64 block boundary.
- Empty-value guard preserved ahead of new forms; secret-order output untouched.
- Whitespace-strip correctly placed as a haystack transform.

**Findings**:
- 🔴 critical (high): Head-prefix of the whole `AUTH_HEADER` value is the header
  name (`Authorizatio`), a substring of ordinary prose; breaks
  `the_header_name_alone_does_not_false_positive` and makes `LOGIN_URL` produce
  `https://exam`. Derive prefixes only from secret-bearing forms; add a
  regression test. (Phase 3, `prefixes()`)
- 🟡 major (medium): `percent_encode` emits uppercase hex only; `%2f` ≡ `%2F` per
  RFC 3986 and lowercase is common, so `a%2fb` evades. Derive both casings or
  normalise the haystack. (Phase 1, `UNRESERVED_ESCAPES` / `percent_encode`)
- 🔵 minor (medium): Only STANDARD base64 derived; URL-safe / unpadded variants
  not caught. Derive them or list under "What We're NOT Doing". (Phase 1,
  `encodings()`)
- 🔵 minor (low): Values 12–15 chars long have an unguarded, undocumented
  head-truncation gap. State it explicitly. (Phase 3 threshold)

### Security

**Summary**: Meaningfully shrinks the false-negative surface (verbatim, STANDARD
base64, maximal percent-encoding, whitespace reflow, head-prefix truncation) and
preserves the load-bearing name-only invariant across every new form. The chosen
encoding set is defensible as a best-effort net but narrower than the threat;
several close cousins of the derived encodings (base64url, no-pad base64, partial
percent-encoding, hex) slip past silently and are not acknowledged in "What We're
NOT Doing". As the last automated barrier before a credential is committed, each
residual encoding is a real leak path worth naming.

**Strengths**:
- Name-only disclosure holds across every new form; each phase re-asserts
  name-present + value-absent.
- The reject-message reword is cosmetic and truthful; no guarantee weakened.
- Char-based slicing avoids a UTF-8 panic (DoS/crash on adversarial input).
- The mid/tail-truncation exclusion is a justified, test-pinned tradeoff.

**Findings**:
- 🟡 major (medium): base64url and no-padding base64 slip past silently and are
  unlisted; the `AUTH_HEADER`/JWT world uses base64url by norm. Derive
  `URL_SAFE`/`URL_SAFE_NO_PAD` or name them. (Phase 1 #2; What We're NOT Doing)
- 🟡 major (medium): Only maximal percent-encoding is derived; ordinary partial
  URL encoding (reserved delimiters left literal) falls between verbatim and
  maximal and matches neither. (Phase 1 #2, `UNRESERVED_ESCAPES`)
- 🟡 major (medium): Hex encoding entirely absent and unlisted; lowercase,
  uppercase and mixed-case hex all slip. (What We're NOT Doing; Desired End State)
- 🔵 minor (low): HTML-entity and JSON `\u` escape forms unaddressed in an
  HTML/JSON-heavy domain. (What We're NOT Doing)
- 🔵 minor (medium): Residual leak surface under-enumerated and the reworded
  doc-comment must not overclaim coverage. (Phase 1 #2, doc-comment)
- 🔵 suggestion (low): Double / nested encoding not considered. (What We're NOT
  Doing)

### Safety

**Summary**: Preserves the fail-safe posture — every new match form funnels
through the same scan → first-offender → exit-1 path, so the "artefact is never
written" guarantee holds, and char-based slicing avoids a UTF-8 panic. But Phase
3's 12-char head-prefix, applied to the whole `Name: value` form and to
low-entropy values, manufactures needles like `Authorizatio` and `https://exam`
that refuse legitimate artefacts. Because refusal is deliberately unrecoverable
(no `--force`), this converts a false positive into an unrecoverable block, and
contradicts the plan's "each phase leaves `mise run` green" claim.

**Strengths**:
- Fail-safe posture preserved; no new artefact-write path.
- UTF-8 panic designed out via char-based length/slicing and char-based
  `strip_whitespace`.
- Empty-value guard retained in the Phase 2 `scan()` rewrite.

**Findings**:
- 🔴 critical (high): Phase 3 derives the head of the whole `AUTH_HEADER` value
  (`Authorizatio`), breaking `the_header_name_alone_does_not_false_positive` and
  refusing any artefact mentioning "Authorization"; Phase 3 cannot be `mise run`
  green. Prefix only the high-entropy value-half / encoded forms; add the
  header-name case to Phase 3's false-positive tests. (Phase 3)
- 🟡 major (high): The 12-char head collides on low-entropy structural values —
  `LOGIN_URL` yields `https://exam`; the whitespace-strip widens the surface
  further. Refusal is unrecoverable, so this is an availability hazard
  outweighing the marginal false-negative reduction for low-entropy values.
  Restrict prefixes to high-entropy encoded forms and add URL/header-name
  false-positive tests. (Phase 3 / What We're NOT Doing)
- 🔵 minor (medium): Broadened matching has no per-form diagnostic; a false
  refusal shows only "in some form". Surface the matched form-class without the
  value. (What We're NOT Doing / Overview)

### Test Coverage

**Summary**: Strongly TDD-shaped — fast domain-unit tests over one integration
test per phase, mirroring the existing behaviour-sentence naming, with
out-of-scope cases and the 12-char prefix boundary pinned as explicit
negative/boundary tests. Two boundary gaps undercut the rigour: the multi-byte
UTF-8 value (the plan's own headline panic risk) has no test, and the ≥16
minimum-form threshold is not pinned at its 15/16 edge.

**Strengths**:
- Test names faithfully mirror the established behaviour-sentence style.
- Out-of-scope decisions pinned as negative tests (mid/tail truncation,
  high-entropy substring).
- The 12-char prefix boundary pinned both sides (P−1 not flagged, P flagged).
- Short-value case covered both directions (no-prefix; prefix via longer base64).
- The `env_clear()` isolation claim holds in `subcommands.rs run()`.
- Balanced pyramid: unit foundation, one integration case per phase.

**Findings**:
- 🔴 critical (high): The flagged multi-byte UTF-8 panic risk has no test — every
  value is ASCII. Add a ≥16-char multi-byte value straddling the 12th-char
  boundary. (Phase 3 Section 2 / Key Discoveries)
- 🟡 major (high): The ≥16 threshold is not pinned at 15/16; a `>= 16` → `>= 15`
  / `> 16` mutation fails no test. Add a boundary pair. (Phase 3 Section 2)
- 🔵 minor (medium): Domain tests do not pair name-present with value-absent as
  the strategy claims. Align wording or tests. (Testing Strategy / Phase 1
  Section 5)
- 🔵 minor (low): Compound shapes only partially pinned (reflow+percent,
  reflow+prefix, prefix-of-header-value-half missing). Add the encoded
  header-value-half prefix case. (Phase 2 & 3 Section 2)

### Code Quality

**Summary**: Lands cleanly on the existing seams, decomposes into pure
independently testable free functions, and is disciplined about char-based
slicing. The maintainability concerns are soft: two load-bearing doc-comments
are updated only in the abstract or silently dropped, and a few helper/const
names misdescribe what they return.

**Strengths**:
- Extends the needle-vs-haystack seam rather than a parallel path.
- Small pure free functions, trivially unit-testable in isolation.
- Explicit char-vs-byte reasoning avoids a non-char-boundary panic.
- Test names mirror the existing convention; assertions pair name/value.

**Findings**:
- 🔵 minor (high): Module doc-comment update specified only in the abstract —
  give the concrete replacement prose for the load-bearing intent statement.
  (Phase 1, Section 2)
- 🔵 minor (medium): The header value-half rationale doc-comment
  (`leaked_credentials.rs:19-24`) is silently dropped when the split moves to
  `values()`; CLAUDE.md says keep exactly this kind of comment — relocate it.
  (Phase 1 Section 2 / Phase 3 Section 1)
- 🔵 suggestion (medium): Helper names misdescribe returns — `encodings()`
  includes the identity form, `prefixes()` returns form+head. Consider `forms()`
  / `form_and_head()`. (Phase 1 / Phase 3)
- 🔵 suggestion (medium): `UNRESERVED_ESCAPES` names the complement of what it
  holds (the escape set). Rename to `PERCENT_ESCAPE_SET`. (Phase 1, Section 2)
- 🔵 suggestion (medium): Homogeneous haystack array forces a needless full-body
  clone; bind `stripped` and use `body.contains(n) || stripped.contains(n)`.
  (Phase 2, Section 1)
- 🔵 suggestion (low): The added `cli/Cargo.toml` comment mostly fits the file's
  why-rationale convention; the trailing usage clause is borderline. (Phase 1,
  Section 1)

### Compatibility

**Summary**: All three dependency claims hold against the actual tree —
`cli/Cargo.lock` carries exactly one `base64` (0.22.1) and one `percent-encoding`
(2.3.2), both dual MIT/Apache-2.0 in `deny.toml`'s allow-list, so caret bounds
add no package and cannot trip `multiple-versions = "warn"`. The public-API
surface is untouched (`needles()` private, `scan()` signature unchanged) and the
exit-code contract is preserved. The only residual gap is that the reject message
is reworded but the published docs still describe the old "literal value"
behaviour.

**Strengths**:
- Dependency claims verified end-to-end against the lockfile
  (`base64` 470-471, `percent-encoding` 3482-3483).
- Single version of each crate graph-wide, so `multiple-versions` has nothing to
  fire on.
- Both licences in `deny.toml`'s blanket allow-list; no exception needed.
- Caret bounds consistent with the workspace convention for behaviour-stable
  crates.
- Public-API and exit-code contracts genuinely stable.
- The reworded reject message is not machine-consumed — no hook/skill/test parses
  it.

**Findings**:
- 🔵 minor (medium): The published command reference
  `docs-site/src/content/docs/design.md:146` still states scrub-secrets refuses
  "the literal value", now inaccurate once encoded/reflowed/truncated forms are
  caught. Add the docs reword to Phase 1; `docs:check` is outside the aggregate
  `check`, so verify it directly. (Phase 1, Section 4)

## Re-Review (Pass 2) — 2026-09-11

**Verdict:** REVISE

Six lenses re-run (correctness, safety, security, test-coverage, code-quality,
compatibility) against the revised plan. Both prior criticals and all four prior
majors are resolved; the revision is monotonic (widening only adds needles,
never regresses to fail-open) and compatibility-clean. But the Phase 3 fix
introduced a new class of defect: `header_value_half()` splits on the first
colon **unconditionally**, so `LOGIN_URL` and colon-bearing passwords are
mistreated as headers — one new high-confidence false positive and one false
negative. The plan needs one more iteration on the prefix-source design before
implementation.

### Previously Identified Issues

- 🔴 Safety + Correctness — Whole-header head-prefix `Authorizatio` breaks an
  existing test — **Resolved**. Prefixes now derive from `prefix_source()`, never
  the whole `Name: value`; `the_header_name_alone_does_not_false_positive` stays
  green, so Phase 3 can leave `mise run` green.
- 🔴 Test Coverage — Multi-byte UTF-8 panic risk untested — **Resolved** (test
  added), but its fixture must place byte offset 12 mid-codepoint to actually
  guard byte-slicing (see new issues).
- 🟡 Security + Correctness — base64url / no-padding base64 not derived —
  **Resolved**. `forms()` derives all four engines.
- 🟡 Security + Correctness — percent-encoding hex casing / partial encoding —
  **Partially resolved**. Lowercase hex derived correctly; partial encoding
  documented as excluded.
- 🟡 Security — hex absent and unlisted — **Resolved** (documented in "What We're
  NOT Doing").
- 🟡 Test Coverage — 15/16 threshold unpinned — **Resolved** (boundary pair added).
- 🔵 Doc-comment prose, value-half rationale relocation, docs-site reword, naming
  (`forms`/`PERCENT_ESCAPE_SET`/`head_prefix`), `scan()` clone, strategy wording,
  architecture boundary/caller notes — **Resolved**.

### New Issues Introduced

- 🟡 Correctness (high) — `LOGIN_URL` always contains a scheme colon, so its
  head-prefix derives from the host substring (`//prototype.`), false-positiving
  on any artefact naming the host (likely, since the artefact documents that
  prototype).
- 🟡 Correctness (medium) — a colon-bearing `PASSWORD` derives its head-prefix
  from the post-colon segment; a head-truncated whole value (`foo:barbazlon`)
  matches nothing — a false negative in the exact truncation case Phase 3 exists
  for.
- 🟡 Safety (medium) — the value-half prefixes (`//example.co`, `Bearer abc12`)
  survive and, with unrecoverable refusal, can deadlock legitimate artefacts; the
  plan does not yet document this residual.
- 🟡 Security (medium) — the derived percent form is maximal-only, the least
  likely real rendering for `LOGIN_URL`, so percent coverage for that variable is
  near-zero; the module doc risks overclaiming.
- 🟡 Test Coverage (medium) — the multi-byte test can pass for the wrong reason
  unless its value places byte offset 12 inside a multi-byte codepoint.
- 🔵 Correctness — an empty/blank header value-half falls back to the whole value,
  reintroducing the structural prefix.
- 🔵 Security — base64-alignment (credential embedded in a larger base64 blob) and
  reflow of a whitespace-bearing needle are undocumented residuals.
- 🔵 Test Coverage — widened-encoding tests should assert form distinctness; the
  `percent_forms` dedup branch is untested.
- 🔵 Safety — Phase 2's whole-document whitespace collapse can reconstitute a
  short common value (`USERNAME=admin`) across word boundaries.

### Assessment

A clear net improvement — the test-breaking critical and the encoding gaps are
genuinely closed — but not yet ready. All three correctness/safety majors share
one root cause: the unconditional first-colon split in `header_value_half()`.
Resolving that (restrict the value-half/prefix-source substitution to the genuine
header variable, or derive head-prefixes only from high-entropy encoded forms)
dissolves the new false-positive, false-negative, and empty-half issues together.
The remaining items are cheap doc/test refinements.

## Re-Review (Pass 3) — 2026-09-11

**Verdict:** REVISE

Four lenses re-run (correctness, safety, security, test-coverage) after the plan
adopted encoded-forms-only head-prefixing. Every pass-2 finding is resolved — the
LOGIN_URL false positive, the colon-password false negative, the empty-half edge,
the value-half deadlock, the multi-byte test (now moot: `head_prefix` slices only
ASCII), and all the doc/test refinements. But three lenses converge, at high
confidence, on a new defect that reveals the encoded-only premise itself is
unsound for the two structured variables: head-prefix soundness is per-variable,
which no uniform rule captures. Phase 3 needs to be dropped or made
variable-aware.

### Previously Identified Issues

- 🟡 Correctness — `LOGIN_URL` host-prefix false positive — **Resolved** (no raw
  value is head-prefixed).
- 🟡 Correctness — colon-`PASSWORD` false negative — **Resolved** (the colon split
  no longer gates any prefix; the whole value always flows through derivation).
- 🔵 Correctness — empty value-half fallback — **Resolved** (`prefix_source`
  removed).
- 🟡 Safety — value-half prefixes (`//example.co`, `Bearer abc12`) deadlock —
  **Resolved** (raw value-half not prefixed).
- 🟡 Test Coverage — multi-byte test may pass for the wrong reason — **Resolved by
  removal** (`head_prefix` only ever slices ASCII encoded forms; confirmed
  panic-free, no code path reaches a multi-byte slice).
- 🟡 Security — maximal-percent overclaim — **Resolved** (module doc bounded to
  "maximal percent-encoding (unreserved set, either hex casing)"; partial out of
  scope).
- 🔵 Security — base64-alignment, reflow-of-whitespace-needle undocumented —
  **Resolved** (enumerated in "What We're NOT Doing").
- 🔵 Test Coverage — distinctness asserts, dedup-branch test — **Resolved** for
  whole forms (but see the new head-prefix-coincidence gap).

### New Issues Introduced

- 🔴 Correctness + Safety (high) — **Encoded head-prefixes of the structured
  variables are structural, not high-entropy.** `base64("Authorization: Bearer
  …")` always begins `QXV0aG9yaXph` (base64 of "Authoriza"); `base64("https://…")`
  always begins `aHR0cHM6Ly9`; percent-encoding never escapes alphanumerics, so
  the percent form of `AUTH_HEADER` keeps the literal head `Authorizatio`. Each is
  a 12-char needle that false-positives on unrelated content base64-ing an auth
  header / URL, or on prose containing "Authorization" — the exact hazard the
  revision claimed to eliminate, reappearing in encoded form. Unguarded:
  `a_structural_raw_head_does_not_false_positive` tests only the raw strings.
- 🟡 Security (as minor, deferring the over-refusal manifestation to
  correctness/safety) — the "encoded heads carry secret bytes" rationale holds
  only for base64 of high-entropy values, not for percent-encoding nor for the
  structural base64 stems.
- 🟡 Security — **encoded-only over-corrected**: `PASSWORD`/`USERNAME` have
  high-entropy heads, so dropping raw head-prefixing loses realistic
  truncated-raw-secret coverage for them with negligible false-positive risk.
- 🟡 Test Coverage — the whole-form distinctness asserts do not guard Phase 3
  head-prefix coincidence (no-pad and padded base64 share the leading 12), so a
  ≥16-char encoded fixture can pass for the wrong reason.
- 🔵 Test Coverage — `a_short_value_gains_a_prefix_through_its_longer_base64` must
  render only the leading 12, else it degenerates into a whole-base64 test.
- 🔵 Safety — the Phase 2 whitespace-collapse residual is framed as
  "mitigated by refusal being safe", conflating leak-safety with the
  availability cost of an unrecoverable false positive.

### Assessment

Encoded-only prefixing fixed everything pass 2 raised, but the deeper lesson is
now unmistakable across three rounds: head-prefix matching is sound only where a
value's *head* is high-entropy — true for `PASSWORD`/`USERNAME`, false for the
`Authorization:` and `https://` structural prefixes in **every** encoding. No
single uniform rule serves all four variables. Two clean resolutions remain: drop
Phase 3 (ship the encoding + reflow phases, which have passed) or make
head-prefixing variable-aware (the adapter marks which values have high-entropy
heads; prefix raw and encoded forms only for those). Phases 1 and 2 are ready;
Phase 3 is not.

## Re-Review (Pass 4) — 2026-09-11

**Verdict:** REVISE

Four lenses re-run (correctness, safety, security, test-coverage) against the
colon-proxy design (head-prefix derived, raw and encoded, only when the whole
secret value is colon-free). **Correctness confirms the design is logically sound
and internally consistent** — no design defect. Every remaining major is a
disclosure, documentation, or test-specification issue, and all were addressed in
the follow-up iteration. This is the convergence point: across four rounds the
findings have gone from real defects to the plan's rationale over-selling a
pragmatic proxy.

### Previously Identified Issues

- 🔴 Correctness + Safety — structural encoded head-prefixes (`QXV0aG9yaXph`,
  percent `Authorizatio`) — **Resolved**. The colon gate suppresses all
  head-prefixing for colon-bearing values; correctness verified the mechanics
  (gate on `self.value`, value-half excluded, borrow/move order, char-based
  slicing).
- 🟡 Security — base64 family / lowercase percent / exclusions — **Remain
  resolved**; detection additive and monotonic; name-only invariant intact.
- 🟡 Test Coverage — multi-byte / boundary / coincidence — **Remain resolved**.

### New Issues Introduced (all addressed in follow-up iteration)

- 🟡 Safety + Correctness — the colon is a *coarse* proxy: colon-free ≠
  high-entropy, so a long low-entropy `USERNAME` (`serviceaccount-prod`) is
  head-prefixed and can false-positive. **Addressed**: disclosed as an accepted
  residual in Key Discoveries and "What We're NOT Doing".
- 🟡 Security — colon-bearing ≠ structural: a bare-key `AUTH_HEADER`
  (`X-API-Key: <key>`) and a colon-password have high-entropy heads the gate
  suppresses — an accepted *missed leak*, understated by the "structural in every
  form" framing. **Addressed**: the disclosure now separates the structural
  (`AUTH_HEADER`/`LOGIN_URL`) case from the coarse-proxy missed-leak case.
- 🟡 Security — module doc-comment and docs-site overclaim head-truncation (no
  colon-free bound) and describe it in Phase 1 though it lands in Phase 3.
  **Addressed**: Phase 1 doc-comment scoped to encodings; reflow and colon-free-
  bounded truncation moved to Phases 2/3.
- 🟡 Safety — the reject message names no match shape, so a false-positive refusal
  is undiagnosable with no `--force`. **Addressed** — the author chose to add it:
  new Phase 4 branches the message into a literal vs a transcribed (encoded,
  reflowed, or truncated) subject via a `leak_is_literal` predicate, still without
  printing the value; `scan`'s public signature is unchanged.
- 🟡 Test Coverage — the colon-proxy negatives were not specified to fail under a
  regression (the cited `aHR0cHM6Ly9` was 11 chars, not the real 12-char head;
  the colon-password fixture lacked a ≥16 length). **Addressed**: negatives now
  carry the actual computed leading-12 (asserted 12 chars) and require ≥16; the
  factual base64 error was corrected in the plan and work item.
- 🔵 Test Coverage — char-vs-byte *threshold* unpinned; no end-to-end colon-proxy
  negative; interior-whitespace-needle exclusion unpinned. **Addressed**: added
  `a_short_multibyte_value_gains_no_prefix`,
  `a_colon_bearing_head_is_accepted_end_to_end`, and
  `a_reflowed_interior_whitespace_needle_is_not_reconstituted`.
- 🔵 Safety — scanner fails open when the credential env vars are absent
  (pre-existing). **Addressed**: noted as an out-of-scope limitation in Migration
  Notes.

### Assessment

The design has converged and is sound: correctness gives it a clean bill, and the
pass-4 majors were entirely about honest disclosure of the colon proxy's coarse
edges (false-positive on low-entropy colon-free values; missed leak on
high-entropy colon-bearing values), the doc-comment overclaim / phase-independence
slip, and tightening the regression-pinning of the colon-proxy negatives — all
now applied to the plan and reflected back into work item 0207. The one open item
— naming the match shape in the reject message (raised each safety pass) — has
been decided by the author in favour of adding it, landing as Phase 4 (a
literal-vs-transcribed subject via a new `leak_is_literal` predicate, no value
printed, `scan` signature unchanged). The plan is implementation-ready; Phase 4
itself is a small, self-contained addition that has not been separately reviewed.

## Re-Review (Pass 5) — 2026-09-11

**Verdict:** APPROVE

Final confirmation over the whole plan (correctness, safety, security,
test-coverage, compatibility), with emphasis on the newly-added Phase 4
(name-the-match-shape) and the public-API surface. **All five lenses confirm the
design is sound.** Correctness verified `leak_is_literal` is a provably correct
classifier (and that the toolchain and `&String: Pattern` idiom hold); safety and
security confirmed Phase 4 preserves the fail-safe posture and the name-only
disclosure invariant (a bool predicate, static shape phrases, classification runs
after the reject decision); compatibility verified against the committed baseline
that `scan`'s signature is untouched and `leak_is_literal` is a purely additive
public item. One major and a few small items were found and **fixed in place** in
this pass; nothing outstanding blocks implementation.

### Confirmed Resolved

- 🟢 Design correctness — sound across all four phases; no defect. The colon-proxy
  suppression, char-based slicing, and Phase 4 classifier all verified.
- 🟢 Fail-safe / disclosure — monotonic detection, exit-1/no-write intact,
  value never printed in either message branch.
- 🟢 Public API — `scan` unchanged vs `cli/design/tests/fixtures/public-api.txt`;
  `leak_is_literal` additive; deps and exit-code contract unchanged.

### Found and Fixed This Pass

- 🟡 Test Coverage — `a_literal_leak_names_the_value` could not catch an
  "always-transcribed" regression, since `the value of {name}` is a substring of
  the transcribed message. **Fixed**: the literal case now also asserts the
  message omits `transcribed`, making the two branches mutually exclusive under
  mutation.
- 🔵 Test Coverage — Testing Strategy prose omitted the Phase 4 tests. **Fixed**:
  the Unit Tests bullet now lists the `leak_is_literal` classification and the
  message-branch cases.
- 🔵 Compatibility — `public-api:check` runs on the nightly lane, so `mise run`
  may not flag a stale baseline. **Fixed**: Phase 4's criterion now names
  `mise run public-api:update` explicitly.
- 🔵 Correctness — "Phase 2 could reorder freely" was imprecise (its `scan` uses
  `needle.as_str()`, needing Phase 1's `Vec<String>`). **Fixed**: the wording now
  states Phase 2 must land after Phase 1.

### Left as Disclosed, Non-Blocking

- 🔵 Safety + Security — the scanner fails open when the `ACCELERATOR_BROWSER_*`
  variables are unset (pre-existing, honestly documented in Migration Notes, out
  of scope). Both lenses flag it as the highest-value residual and recommend
  prioritising the noted follow-up work item — no change to this plan required.

### Assessment

The plan is **approved and implementation-ready**. Over five review passes it
went from 2 critical + 4 major real defects to a design every lens signs off on,
with each residual (the colon proxy's two coarse edges, the encoding exclusions,
the fail-open) honestly disclosed and regression-pinned. Recommended first
implementation step: raise the fail-open follow-up work item so the pre-existing
limitation is tracked before this widened detection ships.
