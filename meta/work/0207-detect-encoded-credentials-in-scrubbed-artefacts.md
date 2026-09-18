---
type: "work-item"
id: "0207"
title: "Detect Encoded Credentials In Scrubbed Artefacts"
date: "2026-08-12T23:21:12+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "ready"
kind: "story"
priority: "medium"
parent: "work-item:0196"
derived_from: ["plan:2026-08-11-0196-design-cli-migration"]
relates_to: ["work-item:0196"]
tags: ["design", "security", "secrets"]
last_updated: "2026-09-11T09:57:03+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-737"
---

# 0207: Detect Encoded Credentials In Scrubbed Artefacts

## Summary

As a developer running `accelerator design scrub-secrets`, I want configured
browser credentials detected even when a model has base64-encoded, percent-
encoded, whitespace-reflowed, or truncated them in an artefact, so that a
transcribed secret cannot slip past a scrubber that only matches literal values.

## Context

`accelerator design scrub-secrets` refuses to let an inventory through when it
repeats the literal value of a configured `ACCELERATOR_BROWSER_*` variable. It
reports the variable's name and never its value, so the report is safe to
print, log and commit.

Matching is literal-substring only. The CLI migration widened it once, because
`ACCELERATOR_BROWSER_AUTH_HEADER` holds a whole `Name: value` pair and the
daemon splits it on the first colon — so an artefact rendering just the bearer
token, the likely leakage shape, matched nothing. The value half is now a
needle — a search string the scanner matches against the artefact — of its own.

That closes one shape and leaves the rest. The artefacts being scanned are
model-authored prose describing a browser session, where a credential is at
least as likely to appear base64-encoded (as it would in an `Authorization`
header the model transcribed), percent-encoded (from a URL), reflowed across a
line wrap (whitespace inserted into the token), or truncated after its head — a
long token clipped partway through, its leading characters intact.

## Requirements

- Derive candidate encodings per named value rather than matching the raw value
  alone: base64 and percent-encoding of each configured `ACCELERATOR_BROWSER_*`
  value, and of the header value-half already extracted for
  `ACCELERATOR_BROWSER_AUTH_HEADER`.
- Strip whitespace from the artefact (haystack) side before matching, not from a
  derived needle: a configured value has no interior whitespace, so stripping the
  value would reproduce the literal match and catch nothing. Removing all
  whitespace from the text being scanned reconstitutes a value a model reflowed
  across a line wrap, so its literal or encoded form still matches.
- Add a minimum-length prefix match: for any needle form of 16 characters or
  more, the leading 12 characters are also a needle, so a token clipped after
  its head is caught. Only a token whose leading 12 characters survive is
  caught; one clipped before its 12th character is out of scope. Forms shorter
  than 16 characters are matched whole-form only.
- Restrict the prefix rule to values whose head is high-entropy. A value
  containing a colon (`AUTH_HEADER`, `LOGIN_URL`) has a structural head in every
  form — raw (`Authorizatio`, `https://exam`), base64 (a fixed head, e.g.
  `QXV0aG9yaXph` = base64 of the `Authoriza` prefix) and percent (which never
  escapes alphanumerics) — so head-prefixing it would flag ordinary prose or an
  unrelated encoded header/URL. Gate prefixing on the value
  being colon-free, decided once per secret so the header value-half is excluded
  too.
- Apply the prefix rule to every derived form of a colon-free value — the raw
  value and every derived encoding alike — so a truncated raw, base64, or
  percent-encoded head is caught.
- A match on any encoding or prefix form is a hard refusal — exit 1, artefact
  not written — identical to today's literal match. No downgrade to a warning.
- The report names only the offending variable and never its value, for every
  encoding and every prefix form.

## Acceptance Criteria

- [ ] Base64 and percent-encoding of each named value (and of the header
      value-half) are detected: the command exits 1, no artefact is written, and
      only the variable name is reported. Each encoding's test uses a value
      containing characters the encoding actually transforms, so the derived
      form differs from the raw value.
- [ ] Given a configured value rendered with whitespace inserted mid-token
      across a line wrap, when scrubbed, then the variable is named — artefact-
      side whitespace stripping reconstitutes the split token, catching a
      match the literal scan would miss.
- [ ] Given a colon-free configured value with a form of 16 characters or more,
      when the artefact contains only that form's leading 12 characters, then the
      variable is named — a token clipped after its head is caught by its head.
- [ ] Given a base64-encoded credential truncated after its head, when scrubbed,
      then the variable is named — the 12-character prefix needle is derived for
      every encoding, not only the raw value.
- [ ] Given a colon-free percent-encoded credential whose derived form is 16
      characters or more, truncated after its head, when scrubbed, then the
      variable is named — confirming the prefix rule covers percent-encoding, not
      only base64. Demonstrated with a colon-free value, since `LOGIN_URL` is
      colon-bearing and excluded from prefixing.
- [ ] Given a colon-bearing value (`AUTH_HEADER`, `LOGIN_URL`), when the artefact
      contains the leading 12 characters of any of its forms — raw, base64, or
      percent — then it is not flagged: a structural head is never a prefix
      needle. The test carries the actual leading-12 of each derived form
      (computed, asserted to be 12 characters) so it flips red under a regression.
- [ ] A colon-containing password of 16 or more characters gains no prefix needle
      — the colon gates prefixing off for the whole secret — and is caught only
      whole. A test pins this documented limitation (the >=16 length ensures the
      test would fail if the gate regressed, rather than pass because the form is
      too short to prefix anyway).
- [ ] A derived form shorter than 16 characters gains no prefix needle and is
      matched whole only, and a test asserts this explicitly. Because the gate
      is per form, a short raw value may still gain a prefix needle via a longer
      derived form (e.g. its base64), and a test asserts that too.
- [ ] Given a configured value's form of 16 characters or more (M, so a prefix
      needle exists), when a legitimate artefact substring shares its leading 11
      characters (P−1), then it is not flagged; when a substring shares the
      leading 12 characters (P), then the variable is named — the prefix boundary
      holds at exactly P.
- [ ] A long value truncated in its middle or tail (its head removed) is not
      flagged — mid-token and tail truncation are deliberately out of scope, and
      a test pins this.
- [ ] A match on any encoding or prefix form is a hard refusal (exit 1, artefact
      not written), consistent with a literal match.
- [ ] The report names only the variable and never the value, for every encoding
      and every prefix form.
- [ ] A test asserts a realistic false-positive candidate — a long, high-
      entropy-looking but legitimate substring sharing fewer than 12 leading
      characters with any configured value — is not flagged.
- [ ] `mise run` exits 0.

## Technical Notes

Scanner lives at `cli/design/src/leaked_credentials.rs`; needle derivation
extends `NamedSecret::needles()`, and the match core is `scan()` (literal
`str::contains` today). Thresholds: prefix length `P = 12`, minimum form length
`M = 16`.

Needle derivation, per named value `V` (and, for a `Name: value` header, its
trimmed value-half). Whitespace is handled on the haystack, not the needle:

```text
forms = [V, base64(V), percent_encode(V)]
prefixable = ':' not in V     # colon-free => high-entropy head
for f in forms:
  needles += f
  if prefixable and len(f) >= 16: needles += f[:12]

haystacks = [artefact, whitespace_strip(artefact)]
reject if any needle occurs in any haystack
```

`base64` is the standard RFC 4648 alphabet with padding; `percent_encode`
escapes every byte outside the unreserved set (`A–Z a–z 0–9 - . _ ~`);
`whitespace_strip` removes all ASCII whitespace from the artefact before
matching, reconstituting a token a line wrap split. The whitespace transform
lives in `scan()` on the haystack, not in `needles()`.

The variable set is fixed by `cli/design-adapters/src/environment.rs`
(`CREDENTIAL_VARIABLES` = `AUTH_HEADER`, `USERNAME`, `PASSWORD`, `LOGIN_URL`);
`LOCATION` is deliberately excluded. Refusal is a `Report::rejected` verdict
(exit 1) per `cli/design-cli/src/commands.rs` and `main.rs`; name-only reporting
is enforced by `scan()` returning names only.

## Drafting Notes

- `P = 12` / `M = 16` chosen to keep the prefix a strict, high-entropy subset of
  a genuinely long secret. A shorter prefix risks blocking a legitimate artefact
  on a common substring, and rejection is unrecoverable — there is no override
  flag today.
- Uniform hard refusal retained deliberately: encoded and prefix matches are
  higher-signal than literal ones (a model readily transcribes an encoded
  header), so downgrading them to warnings would invert the risk gradient.
- The prefix rule is applied to every derived form (raw and encoding) of a
  colon-free value, to catch the compound shape — e.g. a line-wrapped truncated
  base64. It is withheld from colon-bearing values (`AUTH_HEADER`, `LOGIN_URL`):
  their head is structural in every form (base64 of `Authorization: `/`https://`
  is a fixed prefix; percent-encoding leaves alphanumerics literal), so a
  head-prefix there would false-positive on ordinary prose or an unrelated
  encoded header/URL rather than catch a leak. A colon is the domain-local proxy
  for "structural head", and it is *coarse* — it errs both ways, and both errors
  are accepted: a colon-free but low-entropy `USERNAME` is still prefixed (a
  possible false positive, accepted as the safe direction), and a colon-bearing
  but high-entropy head — a bare-key header (`X-API-Key: <key>`) or a
  colon-containing password — is not prefixed (an accepted missed leak). A precise
  entropy heuristic is out of scope. (Surfaced by plan review — see
  `plan:2026-09-11-0207-detect-encoded-credentials-in-scrubbed-artefacts`.)
- Mid-token and tail-truncation are deliberately not given their own detection:
  a needle short enough to catch a dropped head reintroduces the false-positive
  risk `P`/`M` exists to avoid.
- A derived form shorter than 16 characters gets no prefix needle. The gate is
  per form, so a short username or password is caught only if it appears
  verbatim — unless its base64 form reaches 16 characters, which then gains a
  prefix of its own. Accepted trade-off; see the too-short acceptance criterion.
- Whitespace is stripped from the artefact, not derived as a needle. A
  configured value has no interior whitespace, so a needle-side `whitespace_strip(V)`
  would equal `V` and catch nothing new; stripping whitespace from the haystack
  instead is what reconstitutes a token a line wrap split.

## Dependencies

- The scanner surface this work extends — `cli/design/src/leaked_credentials.rs`,
  `NamedSecret::needles()`, `scan()`, the `ACCELERATOR_BROWSER_AUTH_HEADER`
  value-half split, and `CREDENTIAL_VARIABLES` in
  `cli/design-adapters/src/environment.rs` — is delivered by Phase 2 of
  `plan:2026-08-11-0196-design-cli-migration` and has landed. This work is not
  blocked.
- This work consumes that migration's `AUTH_HEADER` value-half extraction (the
  first-colon split) as a fixed contract: any change to that extraction shape is
  coupled to this scanner, since the derived encodings take the value-half as
  their input.

## References

- Derived from: `plan:2026-08-11-0196-design-cli-migration` — the CLI migration
  that introduced the header value-half split this work extends.
- Related: 0196
