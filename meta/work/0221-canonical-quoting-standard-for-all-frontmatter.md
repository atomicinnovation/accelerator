---
type: work-item
id: "0221"
title: "Canonical Quoting Standard for All Frontmatter"
date: "2026-08-22T23:41:55+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: story
priority: high
parent: "work-item:0136"
relates_to: ["work-item:0220", "work-item:0227", "adr:ADR-0065", "adr:ADR-0034", "adr:ADR-0033"]
external_id: PP-750
tags: [frontmatter, corpus, document, correctness, migration]
last_updated: "2026-08-30T14:58:52+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---
# 0221: Canonical Quoting Standard for All Frontmatter

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a plugin maintainer — and on behalf of every plugin user whose own corpus is
validated as they work — I want one canonical quoting standard enforced on write,
so non-conformant frontmatter never reaches a repository unnoticed.

Establish one canonical quoting standard for every YAML frontmatter document the
toolchain writes, make the shared renderer emit it, extend the validator to check
it and have each producer skill run that check on the documents it writes, and
migrate the existing corpus into conformance. The
originating defect: the shared renderer delegates to `serde_saphyr`, which
quotes only where YAML syntax demands it, so producers emit typed-linkage
references unquoted and violate ADR-0034 — but nothing runs the validator, so
non-conformant files reach the repository unnoticed. The fix generalises beyond
linkage: all frontmatter, in every `meta/` doc type and the `.accelerator/`
config files, converges on one deterministic style.

## Context

ADR-0034 (accepted, unsuperseded) requires a typed-linkage reference to be a
single quoted YAML string — `"plan:0042"`, never `plan:"0042"`. ADR-0033
requires `id` and timestamps quoted but types `author`, `status`, and `tags` as
bare. Neither mandates a corpus-wide style, and the renderer honours none of it
actively — it hands the whole value tree to `serde_saphyr` with default minimal
quoting.

The violation has a single origin. `cli/document/src/render.rs:36` is the whole
of `emit()`:

    let mut yaml = serde_saphyr::to_string(frontmatter)

Three meta producers reach it — `cli/work-cli/src/create.rs:237` (`work
create`), `cli/work-cli/src/sync_author.rs:154` (the sync write-back), and
`cli/work-cli/src/update.rs:343` (`work update`). Config writes route through the
same renderer via `cli/config-adapters/src/store.rs:242`.

Observed twice on 2026-08-22. `accelerator work create` wrote work item 0220's
`parent` and `relates_to` unquoted, producing the only three
`BAD-LINKAGE-SHAPE` violations in the entire corpus. Separately, a bidirectional
sync's write-back stripped quoting from all 37 work items it touched — and, from
the same minimal-quoting behaviour, refolded three long titles into `>-` block
scalars and collapsed one `last_updated_note` and five `relates_to` arrays onto
single lines. That churn was reverted by hand before committing (PR #76).

The decision recorded by this item: rather than encode only the narrow ADR
rules, adopt one uniform standard — quote every string, timestamp, and any
scalar whose bare form could be misparsed; leave bare only values with an
unambiguous non-string YAML type (integers, booleans, null). Apply it to every
frontmatter document the toolchain writes, `meta/` and `.accelerator/` config
alike. Because the standard is broader than ADR-0033 (which types
`author`/`status`/`tags` bare) and ADR-0034 (linkage only), it must be ratified
in a new, tightly-scoped ADR that overrides only the quoting clauses of ADR-0033
(identity-value shape) and ADR-0034 (linkage quoting) — both parents remaining
accepted for the base schema and the linkage vocabulary they otherwise define —
otherwise the validator would enforce a rule no decision sanctions, inverting the
original defect.

`accelerator corpus frontmatter validate` is run by nothing today: no producer
skill invokes it after writing a document, and it is wired into no `mise` task or
workflow.

## Requirements

**The canonical quoting standard**

- Every frontmatter scalar is double-quoted, except values with an unambiguous
  non-string YAML type — integers, booleans, and null — which remain bare.
- Sequence elements follow the rule per element: a list of strings becomes a
  list of quoted strings, e.g. `tags: ["frontmatter", "corpus"]`.
- Timestamps (`date`, `last_updated`) are quoted; `schema_version` stays bare
  `1` (the validator already requires the bare form).
- The rule is type-driven, not field-name-keyed, so it applies unchanged across
  every `meta/` doc type and the `.accelerator/` config files.

**Reproduction** (the originating linkage defect)

1. `accelerator work create "<title>" bug medium --parent "work-item:0171" --relates-to "work-item:0194"`
2. `accelerator corpus frontmatter validate`

**Expected** — the written file carries `parent: "work-item:0171"` and
`relates_to: ["work-item:0194"]`; validation exits 0.

**Actual** — the file carries both values unquoted; validation exits 1 with one
`BAD-LINKAGE-SHAPE` per key.

The same defect surfaces on the other two write paths, exercised identically by
step 2: `accelerator work update <id>` rewriting any linkage field, and the
bidirectional sync write-back (run via `/sync-work-items`), which reserialises
every touched item's frontmatter through the same renderer.

**Ratify the standard** — ADR-0065 (a tightly-scoped, now-accepted ADR) records
the canonical standard and overrides the quoting clauses of ADR-0033
(identity-value shape) and ADR-0034 (linkage quoting), quoting the overridden
sentences verbatim and linking both via `relates_to`. Both parents stay
`accepted`: supersession is whole-ADR and would orphan the base schema and
linkage vocabulary they respectively define. The toolchain thereby enforces a
ratified decision rather than a tool preference.

**Migrate the corpus** — a meta-directory migration rewrites all existing
frontmatter (every `meta/` doc type and the committed `.accelerator/config.md`)
to the canonical style in one pass, so post-migration every `meta/` document
validates under `corpus frontmatter validate`, config conforms byte-for-byte via
the shared renderer (validating config itself is deferred to 0227), and every
producer rewrite of an untouched field is byte-identical. The `templates/*.md`
skeletons are rewritten to the canonical style in the same effort so the shipped
templates match producer output (the corpus carries zero frontmatter comments
and no CRLF, so re-rendering each file through the canonical emitter is
byte-lossless).

**Additional surfaces (grounded by codebase research)** — the standard-setting
change reaches four surfaces the originating defect did not name:

- Remove the second, independent tag-item quoting rule in `cli/work/src/tags.rs`
  (`needs_quoting`/`format_tag`). Once the renderer double-quotes every string
  element it is dead weight — its output is an intermediate that is re-parsed and
  re-quoted by the renderer — and it must be simplified out with its two tests
  flipped.
- Retire the parallel Python frontmatter-rules surface
  (`tasks/lint/frontmatter_rules.py`) so one canonical standard is enforced by
  one implementation. It is a pure constants/rules library consumed by three
  tests; retirement deletes it and `test_frontmatter_rules.py`, amends
  `test_conformance.py` to re-source three constants from the Rust schema, and —
  critically — **re-homes template validation** (`test_template_frontmatter.py`
  is today the only validator of `templates/*.md`) as a new Rust template-shape
  check exposing a `frontmatter validate-templates` action, since the corpus
  validator structurally rejects template skeletons.
- Quote `Scalar::Float` symmetrically in both emitter and validator, per
  ADR-0065's closed bare set `{integer, boolean, null}`. No float-typed value
  exists anywhere in `meta/` or `.accelerator/`, so this is defensive only.
- The `public-api.txt` snapshots for `cli/corpus`, `cli/document`, `cli/migrate`,
  and `cli/work` are guards that trip when the violation taxonomy, the value-tree
  surface, or the migration registry changes, and must be regenerated.

The visualiser server's status-patch endpoint is **not** a renderer producer: it
splices the `status:` line byte-for-byte via `patch_status`, never calling the
shared renderer, and preserves the value's existing quote style — so the emitter
change does not reach it and the migration's status-quoting is preserved on later
writes with no further work.

**Producer-run validation** — every producer skill that writes or edits a
frontmatter-bearing `meta/` document gains a step that runs `corpus frontmatter
validate` on the document it just wrote or edited and surfaces any violation
before the skill completes. The closed set of in-scope producer skills (21 in
total — an earlier draft miscounted this as 22) is:

- Work items — `create-work-item`, `extract-work-items`, `refine-work-item`,
  `update-work-item`, `stress-test-work-item`, `sync-work-items`,
  `conduct-spike` (spike-outcome write-back).
- ADRs — `create-adr`, `extract-adrs`, `review-adr` (status transition).
- Plans — `create-plan`, `review-plan`, `validate-plan`, `stress-test-plan`,
  `implement-plan` (phase/status writes).
- Reviews — `review-work-item`.
- Notes — `create-note`.
- Research — `research-codebase`, `research-issue`.
- Design artefacts — `inventory-design`, `analyse-design-gaps`.

Two of these are weak structural fits and need explicit handling: `sync-work-items`
delegates every write to the sync engine (no in-prose file the skill itself
writes), and `implement-plan` only ticks plan checkboxes rather than authoring
frontmatter. Both still carry the invocation to satisfy the static coverage
check — `sync-work-items` validates each touched item after the engine run,
`implement-plan` validates the plan after its status/checkbox writes. None
currently permit `corpus frontmatter validate`, so each gains a new
`allowed-tools` rule (`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus
frontmatter validate *)`), matching the plugin-root-prefixed convention already
used for the `design`/`corpus metadata` subtrees.

Explicitly out of scope: the config-writing skills (`configure`, `init`), which
route to `accelerator config validate` (0227) because `corpus frontmatter
validate` rejects config as `INVALID-TYPE`; read-only skills (`list-work-items`,
`paths`, `visualise`, the search/show integration skills); the commit and PR
skills, which write no `meta/` artefact; and the `migrate` skill, which already
runs the validator in-process over its own rewrites. This is the enforcement
mechanism in place of a CI lane, so the step must be added to each in-scope
producer skill's SKILL.md, not only to the CLI.

**Config validation is out of scope** — config frontmatter conforms here via the
shared renderer and the migration, but validating config (frontmatter quoting
plus semantic config correctness) is deferred to work item 0227 (`accelerator
config validate`), which consumes this standard.

**Severity qualifier** — readers tolerate both forms. `accelerator work show`
returns the same scalar from either, PyYAML parses both to identical values, and
no reader misparses an unquoted flow sequence into a mapping. This is a
conformance defect, not data loss. What makes it serious is that the toolchain
defeats the only mechanism enforcing its own architectural contract.

## Acceptance Criteria

- [ ] Given the shared renderer writing any frontmatter scalar, when the file is
      written, then every scalar is double-quoted except values whose type is
      integer, boolean, or null, which remain bare — so timestamps, floats, and
      any other string-round-tripping value are quoted, across `meta/` artefacts
      and `.accelerator/` config alike.
- [ ] Given a file written by `work create`, `work update`, or the sync
      write-back, when `corpus frontmatter validate` runs over it, then it exits
      0.
- [ ] Given a config write via `config-adapters`, when the file is written, then
      its frontmatter conforms to the canonical standard (verified by test).
      Validating existing config files is out of scope — see the separate
      `accelerator config validate` item.
- [ ] Given a conformant file, when a producer rewrites its frontmatter without
      changing a field, then that field's serialisation is byte-identical — no
      refolded titles, no collapsed arrays, no stripped quotes.
- [ ] Given the validator, when it inspects a bare string value on a field the
      standard requires quoted, then it reports a violation; when it inspects a
      bare integer/boolean/null, then it passes.
- [ ] Given each producer skill in the closed in-scope set enumerated under
      Requirements (Producer-run validation), when its SKILL.md is inspected,
      then it contains a step that runs `corpus frontmatter validate` on the
      document it just wrote or edited — the presence of the `corpus frontmatter
      validate` invocation is asserted statically by a test over every named
      SKILL.md, so coverage completeness is a definite pass/fail.
- [ ] Given `corpus frontmatter validate` run on a document whose frontmatter
      violates the standard — the command each in-scope producer skill invokes as
      its final step — then it exits non-zero and emits the specific violation
      (e.g. `BAD-LINKAGE-SHAPE`) on stderr, so the signal a producer skill
      surfaces is itself deterministically verifiable.
- [ ] Given the corpus before the migration, when the migration runs, then every
      existing `meta/` frontmatter document is rewritten to the canonical style
      and subsequently validates under `corpus frontmatter validate`, and every
      `.accelerator/` config document is rewritten to the canonical style and
      conforms byte-for-byte via the shared renderer (verified per AC #3, since
      `corpus frontmatter validate` does not cover config).
- [x] The standard is ratified by ADR-0065, a tightly-scoped accepted ADR that
      overrides the quoting clauses of ADR-0033 and ADR-0034 (both of which
      remain accepted), quotes the overridden sentences verbatim, and links both
      parents via `relates_to`.
- [ ] A regression test drives a linkage-bearing and a plain-string-bearing
      document through the renderer and fails against the current emitter.
- [ ] Given the `templates/*.md` skeletons, when the migration runs, then each
      is rewritten to the canonical quoting style (quoted `id`/`author`/`status`/
      `title`, flow-quoted `tags`, bare `schema_version: 1`, placeholder tokens
      preserved) and validates under the new `frontmatter validate-templates`
      action.
- [ ] Given the Python frontmatter-rules surface is retired, when
      `test:unit:tasks` and `test:integration:conformance` run, then no template
      or conformance coverage is lost — template-shape validation is re-homed to
      a Rust check and `test_conformance.py` re-sources its constants from the
      Rust schema.
- [ ] Given the tag-array mutation path after the renderer quotes every string
      element, when a tag is added or removed, then the emitted `tags` array is
      double-quoted per element with no separate tag-item quoting rule in play.

## Open Questions

- None outstanding. Both prior questions are resolved: config-file validation is
  split into a separate `accelerator config validate` work item (config still
  conforms here via the renderer and migration); the ratifying ADR (ADR-0065, now
  accepted) is a tightly-scoped decision that overrides the quoting clauses of
  ADR-0033 and ADR-0034 rather than superseding either (see Drafting Notes).

## Dependencies

- Blocked by: ADR-0065 (Canonical Frontmatter Quoting Standard), which ratifies
  the standard by overriding the quoting clauses of ADR-0033 and ADR-0034. It is
  now `accepted`, so this blocker is discharged and enforcement tracks a ratified
  decision.
- Blocks: 0227 (`accelerator config validate`), which reuses this standard's
  type-driven quoting predicate and enforces the standard it ratifies over
  config files.
- Relates to: 0220 (the originating linkage defect this fixes).

## Assumptions

- The renderer is wrong and the contract is right. The fix is to make emission
  conform, not to relax the contract.
- One uniform, type-driven emission style is preferable to per-field or
  per-call-site quoting. It is field-agnostic, so it applies unchanged to
  config's untyped frontmatter.
- The migration can rewrite the whole corpus without semantic change: both
  quoted and bare forms parse to identical values, so only the byte
  representation changes.
- `schema_version` remains bare `1`, the single deliberate exception, because it
  is an unambiguous integer and the validator already requires the bare form.
- Repository-wide ongoing conformance is a non-goal, given the no-CI-lane,
  producer-run decision: the enforced guarantee is that every document written or
  edited through an in-scope producer skill validates on completion; a file
  hand-edited outside a skill is caught only when a skill next rewrites it.
- The static SKILL.md coverage check (AC #6) plus the deterministic command-signal
  check (AC #7) are the accepted proxy for producer-run enforcement: a producer
  skill is a prose-driven prompt, so that a skill honours the validator's non-zero
  exit at runtime is not directly, repeatably testable and is not asserted as a
  criterion.

## Technical Notes

The fix point for emission is single: `cli/document/src/render.rs:36`. Make the
emitter type-driven — force double-quotes on every string scalar and each string
element of a sequence, leave integers/booleans/null bare. `serde_saphyr`'s
global minimal/quote-all knob is insufficient: minimal strips `title`/`date`
quotes, quote-all would also quote `schema_version`. A type-driven pass over the
`Yaml` value tree (`cli/document/src/value.rs` `Serialize` impl) is required.

The renderer is shared by all three meta producers and the config write
(`config-adapters/src/store.rs:242` → `document::render`). A type-driven rule
needs no field knowledge, so config's untyped `Node` tree conforms
automatically: string values quote, integer/boolean values stay bare.

The validator (`cli/corpus/src/frontmatter_validation`) checks raw text
lexically. Extending it: a bare value passes only if it matches an
integer/boolean/null literal or a flow collection whose elements recurse; any
other bare scalar is an unquoted-string violation. The existing
`id`/`schema_version`/linkage checks become special cases of the general rule.
This covers the `meta/` corpus only; enforcing the same rule over config files
lands in the separate `accelerator config validate` item.

Enforcement is producer-driven, not a CI sweep: each skill runs `corpus
frontmatter validate` on the specific document it just wrote or edited, at the end
of the edit, and surfaces any violation. This checks every plugin user's corpus
as they work — not only this repository — and keeps each run scoped to the one
changed file. A file hand-edited outside a skill is caught the next time a skill
rewrites it.

The 37-file churn (PR #76) came from minimal quoting refolding long titles into
`>-` block scalars and collapsing arrays. A single deterministic style removes
the churn source; the migration realigns the sync baseline in the same pass,
clearing the locally-modified reads noted after PR #76.

Grounded mechanisms (from the codebase research): the emitter fix is per-value
`serde_saphyr::DoubleQuoted`, which double-quotes in both block and flow position
and disables block-scalar folding for the value — removing the churn structurally
rather than by hand. The validator does not fully fold the field-specific checks:
a bare `id: 0042` parses as the integer 42 (losing zero-padding), and the general
"bare integer is allowed" rule would pass it, so `id` keeps a dedicated
must-be-quoted constraint; `schema_version` (must be bare `1`), timestamps (ISO
format), and linkage (`BAD-LINKAGE-SHAPE`, preserving richer diagnostics) likewise
stay dedicated. The general rule adds a new `UNQUOTED-STRING` code covering every
other string field, skipping those with a dedicated check to avoid double-report.
The migration is mechanical (modelled on `m0006`), re-rendering every `meta/` file
through the now-canonical emitter and validating in-process via the existing
`MigrationContext::validate_frontmatter` port, plus a config pass and the
template rewrite.

## Drafting Notes

- Scope grew from the original linkage-only defect to a corpus-wide quoting
  standard plus migration and config coverage, at the author's direction. The
  originating bug is now one requirement within a standard-setting item, which is
  why the kind is `story` rather than `bug`.
- The chosen standard contradicts ADR-0033, which types `author`/`status`/`tags`
  bare. Enforcing it without a ratifying ADR would re-create the original defect
  in reverse, so an ADR is treated as a prerequisite, not optional.
- The ratifying ADR is a scoped override, not a supersession. Neither ADR-0033
  (whole base schema) nor ADR-0034 (linkage vocabulary) is mostly about quoting,
  and supersession is whole-ADR, so overriding just the two quoting clauses —
  both parents staying accepted — is the correct mechanism.
- `schema_version` is the one field kept bare — the deliberate exception rather
  than a gap in the standard.
- Enforcement is producer-run rather than a CI lane, at the author's direction:
  each skill validates the document it writes on completion, which protects plugin
  users' own corpora as well as this repository. The accepted tradeoff is that a
  file edited outside a skill is not checked until a skill next rewrites it.
- Config validation split out at the author's direction: config frontmatter
  conforms here via the shared renderer and the migration, but validating config
  — frontmatter quoting plus semantic config correctness — is a separate
  `accelerator config validate` work item.
- Sizing: the item spans the emitter, the validator, a corpus-wide migration, and
  the validate step across every in-scope producer skill — an epic-scale surface.
  Kept as a single story at the author's direction: the four core pieces are
  strictly interdependent, and the producer-skill wiring is mechanical and uniform
  across a now-closed, enumerated set (see Requirements), so the breadth is bounded
  rather than open-ended. The producer-skill enforcement wiring is nonetheless a
  distinct deliverable that depends only on the extended validator; if it threatens
  to stall the core emitter/validator/migration fix (and 0220's linkage bug it
  closes), split it into a follow-up child story.
- ADR-0065 has since been accepted, discharging the ratification prerequisite; the
  corresponding acceptance criterion is marked done and the Dependencies updated to
  show the blocker cleared.
- Codebase research (2026-08-30) grew the scope beyond the four core pieces: the
  second tag-item quoting rule, the parallel Python frontmatter-rules surface (with
  template validation re-homed to a new Rust template-shape check), `Scalar::Float`
  symmetry, and the four `public-api.txt` guards. It also corrected the producer
  count (21, not 22) and confirmed the visualiser is not a sixth write path. These
  additions keep the item epic-scale but bounded; the plan sequences them as
  independently mergeable phases.
- Parent kept at `0136`, the in-progress Rust CLI epic. `external_id` PP-750
  preserved.

## References

- ADR-0065: Canonical Frontmatter Quoting Standard (ratifies this decision)
- ADR-0034: Typed linkage vocabulary for meta/ artifacts
- ADR-0033: Unified base frontmatter schema for meta/ artifacts
- Research: `meta/research/codebase/2026-08-30-0221-canonical-frontmatter-quoting-standard.md`
- Plan: `meta/plans/2026-08-30-0221-canonical-frontmatter-quoting-standard.md`
- Related: 0220, 0227, 0136
- PR #76
