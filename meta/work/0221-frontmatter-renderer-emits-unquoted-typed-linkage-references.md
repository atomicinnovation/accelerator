---
type: work-item
id: "0221"
title: "Frontmatter Renderer Emits Unquoted Typed-Linkage References"
date: "2026-08-22T23:41:55+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: bug
priority: high
parent: "work-item:0136"
relates_to: ["work-item:0220", "adr:ADR-0034"]
external_id: PP-750
tags: [frontmatter, corpus, document, correctness]
last_updated: "2026-08-22T23:41:55+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---
# 0221: Frontmatter Renderer Emits Unquoted Typed-Linkage References

**Kind**: Bug
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Every artefact producer that writes frontmatter emits typed-linkage references
unquoted, violating ADR-0034's contract that a reference is a single quoted YAML
string. The shared renderer delegates to `serde_saphyr`, which quotes only where
YAML syntax demands it. The corpus validator rejects the result — but nothing
runs the validator, so non-conformant files reach the repository unnoticed.

## Context

ADR-0034 (accepted, not superseded) is explicit:

> The whole reference is a single quoted YAML string in both forms, per
> ADR-0033's identity-value contract — `"plan:0042"`, never `plan:"0042"`.
> Lists of references are YAML arrays of such strings.

The violation has a single origin. `cli/document/src/render.rs:36` is the whole
of `emit()`:

    let mut yaml = serde_saphyr::to_string(frontmatter)

Three producers reach it — `cli/work-cli/src/create.rs:237` (`work create`),
`cli/work-cli/src/sync_author.rs:154` (the sync write-back), and
`cli/work-cli/src/update.rs:343` (`work update`). Config writes route through the
same renderer via `cli/config-adapters/src/store.rs:242`.

Observed twice on 2026-08-22. `accelerator work create` wrote work item 0220's
`parent` and `relates_to` unquoted, producing the only three
`BAD-LINKAGE-SHAPE` violations in the entire corpus. Separately, a bidirectional
sync's write-back stripped quoting from all 37 work items it touched — and, from
the same minimal-quoting behaviour, refolded three long titles into `>-` block
scalars and collapsed one `last_updated_note` and five `relates_to` arrays onto
single lines. That churn was reverted by hand before committing (PR #76).

`accelerator corpus frontmatter validate` is referenced by no `mise` task, no
file under `tasks/`, and no GitHub workflow.

## Requirements

**Reproduction**

1. `accelerator work create "<title>" bug medium --parent "work-item:0171" --relates-to "work-item:0194"`
2. `accelerator corpus frontmatter validate`

**Expected** — the written file carries `parent: "work-item:0171"` and
`relates_to: ["work-item:0194"]`; validation exits 0.

**Actual** — the file carries both values unquoted; validation exits 1 with one
`BAD-LINKAGE-SHAPE` per key.

**Severity qualifier** — readers tolerate both forms. `accelerator work show`
returns the same `parent` scalar from either, PyYAML parses both to identical
values, and no reader misparses an unquoted flow sequence into a mapping. This
is a conformance defect, not data loss. What makes it serious is that the
toolchain defeats the only mechanism enforcing its own architectural contract.

## Acceptance Criteria

- [ ] Given a producer writing any typed-linkage key (`parent`, `blocks`,
      `blocked_by`, `derived_from`, `relates_to`, `source`), when the file is
      written, then every reference is emitted as a quoted YAML string per
      ADR-0034.
- [ ] Given a file whose frontmatter was written by `work create`, `work
      update`, or the sync write-back, when `corpus frontmatter validate` runs
      over it, then it exits 0.
- [ ] Given a conformant file, when a producer rewrites its frontmatter without
      changing a given field, then that field's serialisation is byte-identical
      — a write-back adds only what it changed.
- [ ] Given the repository at any commit, when `mise run check` runs, then
      `corpus frontmatter validate` runs within it and any violation fails the
      check.
- [ ] A regression test drives a linkage-bearing document through the renderer
      and fails against the current emitter.

## Open Questions

- What is the canonical quoting style? Conformant files quote `title`, `date`,
  `id`, and linkage refs but leave `author`, `status`, and `tags` bare, so
  "quote every string" would itself churn the corpus.
- Is ADR-0034 binding on the config-writing call sites in `config-adapters`, or
  scoped to `meta/` artefacts only?

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- The renderer is wrong and the validator is right. ADR-0034 is accepted and
  unsuperseded, so the quoted form is the contract rather than the validator's
  preference.
- Every producer should converge on one canonical emission style, rather than
  each call site quoting its own fields before handing them to the renderer.

## Technical Notes

The fix point is single: `cli/document/src/render.rs:36`.

The third acceptance criterion carries a real design tension. Byte-identical
rewrite of untouched fields can be met two ways — define a canonical style that
every conformant file already satisfies, so a rewrite is a no-op; or preserve
the source's own quoting. The first is far simpler: `serde_saphyr` round-trips
through a value tree and retains no source tokens, so preservation would mean
replacing the emitter rather than configuring it.

The 37-file churn was reverted in PR #76, but the sync baseline still hashes the
churned content, so those items read as locally-modified until one realigning
run.

## Drafting Notes

- Recorded as `bug` against a ratified ADR rather than a style preference; the
  ADR quotation is reproduced in Context so the claim can be checked without
  opening it.
- Priority set to high at the author's direction despite readers tolerating both
  forms. The reasoning recorded here is that the emitter defeats the enforcement
  mechanism, not that artefacts are unreadable.
- CI wiring folded into this item rather than split out — it is the reason both
  instances went unnoticed.
- Scope covers the wider non-linkage churn (stripped scalar quotes, refolded
  titles, collapsed arrays) as well as linkage quoting, since all of it comes
  from the same delegation.
- Parent set to `0136`, the in-progress Rust CLI epic. `0166` and `0179` built
  the crates involved but are both `done`.
- Reader tolerance was established with `work show` and PyYAML. The claim is
  that values survive intact through both forms, not that every parser produces
  a byte-identical internal representation.

## References

- ADR-0034: Typed linkage vocabulary for meta/ artifacts
- Related: 0220, 0136
- PR #76
