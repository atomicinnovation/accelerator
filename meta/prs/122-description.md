---
type: "pr-description"
id: "122"
title: "[0207] Detect encoded credentials in scrubbed artefacts"
date: "2026-09-13T10:40:33+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0207"
parent: "work-item:0207"
relates_to: ["work-item:0196"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/122"
pr_number: 122
tags: ["design", "security", "secrets"]
revision: "e1c875712851cbbbada388e55977ad94b8a0fd64"
repository: "accelerator"
last_updated: "2026-09-20T18:05:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0207] Detect encoded credentials in scrubbed artefacts

## Summary

`accelerator design scrub-secrets` now catches a configured
`ACCELERATOR_BROWSER_*` credential even when a model transcribed it into an
encoded, line-wrapped, or head-truncated shape, closing the gap where
literal-substring matching let a transcribed secret through. The artefacts
scanned are model-authored prose, where a credential is as likely to appear
base64-encoded, percent-encoded, or reflowed as verbatim. The report still
names only the offending variable — never its value or any derived form — so it
stays safe to print, log, and commit.

## Changes

- **Encoding derivation** (`cli/design/src/leaked_credentials.rs`): each
  configured value, and the `AUTH_HEADER` value-half, now derives base64
  (standard and URL-safe, padded and unpadded) and maximal percent-encoding
  (both hex casings) needles alongside the raw value.
- **Haystack whitespace stripping**: the body is matched both as-is and with all
  ASCII whitespace removed, reconstituting a value a model reflowed across a line
  wrap. Stripping the body rather than the needle is deliberate — a configured
  value has no interior whitespace, so stripping the needle would just reproduce
  the literal match.
- **Head-prefix rule**: any needle form of 16 characters or more also
  contributes its leading 12 characters, catching a token clipped after its
  head. Prefixing is gated on the value being colon-free, decided once per
  secret so the header value-half inherits the refusal — a structural head
  (`AUTH_HEADER`, `LOGIN_URL`) would otherwise false-positive on ordinary prose
  or an unrelated encoded header/URL.
- **Rejection wording**: a new `leak_is_literal` lets `scrub-secrets`
  distinguish a verbatim leak (`the value of NAME`) from a transcribed one
  (`a transcribed form (encoded, reflowed, or truncated) of the value of NAME`).
  Both remain a hard refusal — exit 1, artefact not written.
- **Dependencies and boundaries**: `base64` and `percent-encoding` added to the
  workspace and the `design` crate; the cargo-pup domain-import allowlist widens
  for both, since each is a zero-dependency, pure, no-I/O crate that drags no
  transitive graph into the domain.
- **Docs** (`docs-site/src/content/docs/design.md`): the `scrub-secrets` section
  now states the in-scope shapes and the out-of-scope ones explicitly.

## Context

Implements work item **0207** (`Detect Encoded Credentials In Scrubbed
Artefacts`, `PP-737`), under epic 0196. The plan, codebase research, and both
reviews are included in the diff under `meta/`.

## Testing

- [x] `cargo test -p design -p accelerator-design` — 151 domain, 7 command, and
      45 `subcommands.rs` end-to-end tests pass; 0 failed.
- [x] Encoding shapes covered: standard/URL-safe base64, padded/unpadded,
      upper/lower percent triplets, the `AUTH_HEADER` value-half in each.
- [x] Boundary cases covered: 12-character prefix cutoff, the 16-character
      minimum form length, multibyte (char- not byte-indexed) values, and the
      colon-bearing / structural-head refusal that guards against
      false positives.
- [x] Read-only CI lanes re-verified during validation: `mise run cli:check`,
      `deny:check`, `pup:check`, and `public-api:check` all green.
- [ ] Full `mise run` default and `docs:check` — not re-run (the default
      reformats in place; docs needs network + a Chromium install). The plan
      records both green from the implementation session.

## Notes for Reviewers

- **Validated (pass)**: all four phases are fully implemented. The only
  deviations are a plan-only package-name inaccuracy (`design-cli` →
  `accelerator-design` in the plan's success-criteria commands) and a required
  `pup.ron` allowlist widening the plan did not enumerate; neither affects the
  shipped behaviour.
- **Out of scope, by design**: case folding, hex, partial percent-encoding,
  HTML/JSON escapes, nested encodings, and head truncation of a colon-bearing
  value. These are stated in the module doc comment and the user-facing docs so
  the boundary is explicit rather than accidental.
- **False-positive guard** rests entirely on the colon heuristic for deciding a
  head is structural. Worth a look: whether any colon-free, high-entropy value
  could still collide with unrelated prose on a 12-character prefix.
- **Prefix arithmetic is char-based**, not byte-based, so multibyte values are
  measured by character count; the `€`-repeat tests pin this.
- **Fail-open when unset**: with no `ACCELERATOR_BROWSER_*` configured there are
  no needles, so every artefact passes. Out of scope here, flagged as a
  follow-up candidate.
