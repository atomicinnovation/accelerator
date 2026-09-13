---
type: "plan-validation"
id: "2026-09-11-0207-detect-encoded-credentials-in-scrubbed-artefacts-validation"
title: "Validation Report: Detect Encoded Credentials In Scrubbed Artefacts Implementation Plan"
date: "2026-09-20T17:56:52+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-11-0207-detect-encoded-credentials-in-scrubbed-artefacts"
tags: ["design", "security", "secrets"]
last_updated: "2026-09-20T17:56:52+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Detect Encoded Credentials In Scrubbed Artefacts

All four phases are fully implemented, one commit each, and every read-only CI
lane re-run this session exits 0. The scanner now derives base64 (four engines)
and percent-encoding (both hex casings) needles, reconstitutes line-wrapped
tokens against a whitespace-stripped haystack, head-prefixes colon-free values
at a 16-character floor, and names the leak shape (literal vs transcribed)
without printing the value. Two benign deviations (a plan-only package-name
inaccuracy, and a required `pup.ron` allowlist widening the plan did not
enumerate) and one plan-acknowledged fail-open limitation are the only findings;
none blocks the verdict.

### Implementation Status

- ✓ Phase 1: Encoding derivation and form-agnostic reporting — Fully implemented
- ✓ Phase 2: Whitespace-reflow reconstitution — Fully implemented
- ✓ Phase 3: Minimum-length head-prefix matching — Fully implemented
- ✓ Phase 4: Name the match shape in the rejection — Fully implemented

Each phase landed as its own change: `vrxlwpmm` (Phase 1), `kqystlxl` (Phase 2),
`zpkrlqwk` (Phase 3), `qtspkpoz` (Phase 4).

### Automated Verification Results

Re-run this session against the working copy:

- ✅ Domain + command tests: `cargo test -p design -p accelerator-design` —
  151 domain, 7 command, 45 subcommands, all fixtures; 0 failed
- ✅ Rust format + clippy: `mise run cli:check` — clean
- ✅ Dependency audit: `mise run deny:check` — advisories, bans, licenses,
  sources all ok (new direct deps admitted, no duplicate-version warning)
- ✅ Architecture: `mise run pup:check` — clean (the `design` domain allowlist
  now admits `base64`/`percent_encoding`)
- ✅ Public API: `mise run public-api:check` — clean (baseline matches the
  additive `leak_is_literal`)

Not re-run this session, with the reason:

- The full `mise run` default and `mise run docs:check` were not re-executed:
  the default reformats in place and both need network + a Chromium install. The
  docs prose change was verified by inspection instead; the frontend and server
  components are untouched by this CLI-only change. The plan records both as
  green from the implementation session.

### Code Review Findings

#### Matches Plan

- `needles()` widened to `Vec<String>`, with `values()`,
  `header_value_half()` and `head_prefixable()` split out exactly as specified
  (`cli/design/src/leaked_credentials.rs:39-93`).
- `encodings()` derives the four base64 engines plus `percent_forms()` in both
  hex casings, with the `lower == upper` dedup branch intact
  (`leaked_credentials.rs:95-127`).
- `scan()` builds one whitespace-stripped haystack outside the secret loop and
  tests each needle against both copies (`leaked_credentials.rs:132-149`).
- `head_prefix()` is character-based (`chars().count()`/`chars().take()`), gated
  on `MINIMUM_FORM_LENGTH = 16` and `PREFIX_LENGTH = 12`, and applied only to a
  colon-free value (`leaked_credentials.rs:76-93`).
- `leak_is_literal()` is the sole new public item; `scan()`'s signature is
  unchanged, and the message branches literal vs transcribed without printing
  the value (`leaked_credentials.rs:156-162`, `cli/design-cli/src/commands.rs:92-107`).
- Every domain and integration test named in the plan is present and passing,
  including the char-vs-byte, colon-proxy and boundary cases.
- The shipped command reference (`docs-site/src/content/docs/design.md:146-160`)
  drops "literal" and enumerates the encoded, reflowed and colon-free
  head-truncated forms, matching the module doc-comment.

#### Deviations from Plan

- ⚠️ The plan's success-criteria commands name the CLI crate `design-cli`; the
  actual package is `accelerator-design` (`cli/design-cli/Cargo.toml:2`). Every
  `-p design-cli` invocation in the plan fails ID resolution and must be run as
  `-p accelerator-design`. Plan-text inaccuracy only — the implementation is
  correct.
- `cli/pup.ron` gained `^base64(::|$)` and `^percent_encoding(::|$)` to the
  `design_domain_imports_only_permitted` rule. This is required for `pup:check`
  once the domain takes its first external deps, but the plan's "Changes
  Required" did not enumerate it (it only referenced cargo-pup generally). The
  addition is correct and its rationale comment is sound.
- The `cli/Cargo.toml` dependency comment is expanded beyond the plan's
  suggested wording (it names the `multiple-versions = "warn"` interaction and
  the locked 0.22.1/2.3.2 versions). An improvement, not a regression.

#### Potential Issues

- The scanner fails open when the `ACCELERATOR_BROWSER_*` variables are unset —
  no configured secret means no needles, so every artefact passes. Documented in
  the plan's Migration Notes and out of scope, but the widened detection gives
  zero protection in that state; a variable rename or dropped export silently
  disables it. A genuine follow-up candidate.
- The accepted false-positive residuals (whitespace reconstitution of a short
  low-entropy value; head-prefixing a long low-entropy `USERNAME`) are
  unrecoverable refusals — there is no `--force`. The availability cost is
  documented and accepted, not mitigated.

### Manual Testing

Performed this session through the built binary with a cleared environment:

- ✅ base64-encoded `PASSWORD` → exit 1, variable named, transcribed-shape
  phrasing, neither the raw value nor the encoding printed; with no configured
  secret the same body exits 0 (the documented fail-open).
- ✅ colon-free `PASSWORD` (≥16) rendered as only its leading 12 → exit 1;
  colon-bearing `AUTH_HEADER` head (`Authorizatio`) → exit 0.
- ✅ verbatim value → exit 1 with "the value of {name}…" and no "transcribed".

No further manual testing is required; the integration suite exercises each
headline behaviour end-to-end.

### Recommendations

- Correct the plan's success-criteria package name from `design-cli` to
  `accelerator-design` if the plan is kept as a reference (traceability nit).
- Raise a follow-up work item for the unset-variable fail-open: validate that
  the expected credential variables are present when scrubbing is meant to be
  active.
