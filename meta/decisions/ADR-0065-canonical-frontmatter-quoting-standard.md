---
type: "adr"
id: "ADR-0065"
title: "Canonical Frontmatter Quoting Standard"
date: "2026-08-28T17:17:31+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
decision_makers: ["Toby Clemson"]
parent: "work-item:0221"
relates_to: ["adr:ADR-0033", "adr:ADR-0034"]
tags: ["frontmatter", "yaml", "corpus", "config"]
last_updated: "2026-08-30T01:06:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0065: Canonical Frontmatter Quoting Standard

**Date**: 2026-08-28
**Status**: Accepted
**Author**: Toby Clemson

## Context

Frontmatter across the corpus is written by a shared renderer
(`cli/document/src/render.rs`) that delegates to `serde_saphyr` with default
minimal quoting — it quotes only where YAML syntax demands. Two accepted ADRs
mandate narrow quoting rules: ADR-0033 requires `id`, foreign `<type>_id`, and
timestamps quoted but types `author`, `status`, and `tags` as bare strings;
ADR-0034 requires typed-linkage references quoted. The renderer honours neither
actively, so producers emit unquoted linkage references (violating ADR-0034),
and the only tool that could catch it — `corpus frontmatter validate` — never
sees producer output: no producer invokes it after writing a document, and the
one CI lane that runs it validates only synthesised fixtures, never the live
corpus. The result is divergent quoting across the corpus and non-conformant
files reaching the repository unnoticed (work item 0221).

The narrow rules are also hard to enforce mechanically. "Quote `id`, timestamps,
and linkage refs but leave `author`/`status`/`tags` bare" is a per-field
contract; a value-tree serializer's global minimal/quote-all knob cannot express
it, and the corpus already carries mixed styles from minimal quoting — long
titles refolded into `>-` block scalars, arrays collapsed, quotes stripped on
write-back (reverted by hand in PR #76).

## Decision Drivers

- A single deterministic emission style, so rewriting an untouched field yields
  byte-identical output and no incidental churn.
- Enforceability by both the renderer and the validator without per-field
  special-casing.
- One standard for every frontmatter document the toolchain writes — `meta/`
  artefacts and `.accelerator/` config alike.
- Ambiguity resistance: values that YAML's implicit typing could misparse (a
  string that looks numeric, a colon-bearing reference) must survive round-trips
  intact.

## Considered Options

1. **Honour the existing narrow ADR rules** — quote only `id`, timestamps, and
   linkage refs; leave `author`/`status`/`tags` bare. Matches ADR-0033/0034 as
   written, but is a per-field contract the renderer cannot express with a global
   knob, and leaves the corpus's mixed non-linkage quoting unresolved.
2. **Minimal quoting (serde_saphyr default)** — quote only where YAML syntax
   demands. Already the behaviour and simplest, but it is the source of the bug:
   linkage refs and string-typed values go bare, and it churns the corpus
   (strips `title`/`date` quotes, refolds long scalars).
3. **Type-driven canonical standard** — quote every string scalar (and each
   string element of a sequence) and every timestamp; leave bare only values
   with an unambiguous non-string type (integers, booleans, null). Field-agnostic,
   so it applies unchanged to config's untyped frontmatter and needs no per-key
   logic. Broader than ADR-0033/0034, so it requires overriding their quoting
   clauses.
4. **Quote every scalar (global quote-all)** — flip `serde_saphyr`'s global knob
   so even integers, booleans, and null are quoted. The simplest possible emitter
   — one setting, no type inspection — but it quotes `schema_version` to `"1"`,
   breaking ADR-0033's integer contract and the validator's bare-`1` requirement,
   and quotes every boolean and null for no round-trip benefit.

## Decision

We will adopt option 3. Every frontmatter scalar the toolchain writes is
double-quoted, except values whose type is one of a closed set — integer,
boolean, null — which stay bare. The set is closed deliberately: a timestamp or
float is quoted even though YAML's implicit typing would read it as non-string,
so any value that must round-trip as a string never goes bare on a technicality.
Sequence elements follow the rule per element. `schema_version` is the only
integer-typed field, so it lands bare by this rule rather than as an exception to
it. The standard applies to every frontmatter document the toolchain writes: all
`meta/` doc types and the `.accelerator/` config files.

This standard is broader than the accepted quoting clauses of ADR-0033 and
ADR-0034, and **overrides those two clauses specifically**:

- ADR-0033's identity-value shape contract — "The base `id` field is always a
  quoted YAML string …" with `author`/`status`/`tags` typed bare — is
  generalised: all string values are quoted, not only identity values.
- ADR-0034's linkage-quoting rule — "The whole reference is a single quoted YAML
  string … `"plan:0042"`, never `plan:"0042"`" — is subsumed as a special case
  of the general rule.

We do **not** supersede either ADR. Each remains accepted and authoritative for
its primary concern — ADR-0033 for the unified base schema (fields, provenance,
`schema_version`, per-type extras), ADR-0034 for the typed-linkage vocabulary
(keys, the type-pair table, reference forms). Supersession is whole-ADR and would
orphan the majority of each that has nothing to do with quoting. This ADR links
to both via `relates_to` and quotes the exact overridden sentences so a reader of
either parent can find what changed.

## Consequences

### Positive

- One deterministic style: rewriting an untouched field emits byte-identical
  output, ending the write-back churn (refolded titles, collapsed arrays,
  stripped quotes).
- Enforceable symmetrically by the renderer (emit the style) and the validator
  (a bare value passes only if it is an integer/boolean/null literal or a flow
  collection whose elements recurse), with no per-field knowledge.
- Field-agnostic, so config's untyped frontmatter conforms through the same
  shared renderer with no config-specific logic.
- Ambiguity-proof: string values that look numeric or carry colons survive
  round-trips because they are always quoted.
- Enforcement can be producer-driven: because the same validator runs on a single
  written document, each skill checks its own output on completion — so the
  standard is enforced in every plugin user's corpus as they work, not only in
  this repository.

### Negative

- Contradicts the as-written text of two accepted ADRs. A reader landing on
  ADR-0033 or ADR-0034 alone sees the old bare-field guidance and must follow the
  `relates_to` edge to discover the override — the vocabulary has no formal
  partial-supersession edge, so the correction lives in prose here.
- Requires a one-off corpus migration; until it runs, the corpus is mixed.
- Slightly noisier frontmatter — previously bare fields (`author`, `status`,
  `tags`) gain quotes.
- Producer-run validation covers only documents written through a skill; a
  hand-edited or externally-generated file is not checked until a skill next
  rewrites it.

### Neutral

- No semantic change: quoted and bare forms parse to identical values, so the
  migration is a byte-representation change only.
- `schema_version` stays bare, matching the validator's existing requirement —
  it is the only integer-typed field, so the rule itself keeps it bare and no
  special case is needed.
- Enforcement wiring (the emitter, validator, migration, and the skill-time
  validation each producer runs on the documents it writes) is owned by work item
  0221, not this ADR.

## References

- `work-item:0221` — Canonical Quoting Standard for All Frontmatter (owning work
  item; implements this decision)
- `adr:ADR-0033` — Unified base frontmatter schema (identity-value shape clause
  overridden here)
- `adr:ADR-0034` — Typed linkage vocabulary (linkage-quoting clause subsumed here)
- `work-item:0227` — accelerator config validate (enforces this standard over
  config)
- PR #76 — the reverted 37-file write-back churn that motivated a deterministic
  style
