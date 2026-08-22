---
type: inventory
id: "0211-fixture-reconciliation"
title: "0211 Fixture Reconciliation Ledger"
date: "2026-08-22T00:00:00+00:00"
author: Toby Clemson
work_item_id: "work-item:0211"
parent: "work-item:0211"
tags: [jira, linear, integrations, fixtures, cutover]
schema_version: 1
---

# 0211 Fixture Reconciliation Ledger

Every bash-cluster fixture is either carried into the Rust corpus (where a Rust
test consumes it) or ledgered here with a reason, per Decision 15. Silence is
impossible: each section's row count is pinned against the pre-deletion file
list, and a "ported" row means "consumed" — a Rust test drives it.

Sections fill in as their phase lands:

- **ADF samples** (Phase 0): the 43 `skills/integrations/jira/scripts/test-fixtures/adf-samples/` files reconciled against the 56 committed `cli/jira-client/tests/fixtures/adf/` cases.
- **Linear scenarios** (Phase 1/2): the 40 `skills/integrations/linear/scripts/test-fixtures/scenarios/` files.
- **Jira scenarios** (Phase 3/4): the 95 `skills/integrations/jira/scripts/test-fixtures/scenarios/` files.

---

## ADF samples — 43 files (Phase 0)

Disposition of every file under
`skills/integrations/jira/scripts/test-fixtures/adf-samples/`. A "represented"
row names the committed case in `cli/jira-client/tests/fixtures/adf/` that
already exercises the same condition; a "ported" row names the new case that
consumes it; a "dropped" row states why porting adds no differential coverage.

| File | Disposition | Represented by / ported as / reason |
|---|---|---|
| `bold-italic-asterisk.md` | represented | `assemble-inline-priority` (`**strong** *em*`) |
| `bold-italic-asterisk.adf.json` | represented | `assemble-inline-priority` |
| `bold-italic-code-link.md` | represented | `assemble-inline-priority` (code + strong + em + link) |
| `bold-italic-code-link.adf.json` | represented | `assemble-inline-priority` |
| `bullet-list-flat.md` | represented | `assemble-bullet-list` |
| `bullet-list-flat.adf.json` | represented | `assemble-bullet-list` |
| `checklist-mixed.md` | represented | `assemble-task-list` |
| `checklist-mixed.adf.json` | represented | `assemble-task-list` |
| `code-block-no-lang.md` | represented | `assemble-code-fence-no-language` |
| `code-block-no-lang.adf.json` | represented | `assemble-code-fence-no-language` |
| `code-block-with-lang.md` | represented | `assemble-code-fence` (language variant) |
| `code-block-with-lang.adf.json` | represented | `assemble-code-fence` |
| `crlf-input.md` | represented | `assemble-crlf-line-endings` |
| `empty-doc.md` | represented | `assemble-empty-input` |
| `empty-doc.adf.json` | represented | `render-empty-doc` |
| `hard-break.md` | represented | `assemble-hard-break` |
| `hard-break.adf.json` | represented | `assemble-hard-break` |
| `headings-h1-to-h6.md` | represented | `assemble-headings` |
| `headings-h1-to-h6.adf.json` | represented | `assemble-headings` |
| `inline-combinations.md` | represented | `assemble-link-nested-marks` (link wrapping marks) |
| `inline-combinations.adf.json` | represented | `assemble-link-nested-marks` |
| `mixed-asterisk-emphasis.md` | represented | `assemble-inline-priority` (`***both***`) |
| `mixed-asterisk-emphasis.adf.json` | represented | `assemble-inline-priority` |
| `mixed-everything.md` | represented | constituent elements covered by `assemble-headings`, `assemble-inline-priority`, `assemble-mixed-lists-flush`, `assemble-code-fence`, `assemble-hard-break`; the integration-of-all adds no differential coverage the parts do not |
| `mixed-everything.adf.json` | represented | as above |
| `ordered-list-flat.md` | represented | `assemble-ordered-order-always-one` |
| `ordered-list-flat.adf.json` | represented | `assemble-ordered-order-always-one` |
| `paragraph-only.md` | represented | `assemble-paragraph` |
| `paragraph-only.adf.json` | represented | `assemble-paragraph` |
| `placeholder-collision.md` | **ported** | `assemble-placeholder-collision` (input.md) — literal text matching the placeholder format round-trips; no committed case exercised it |
| `placeholder-collision.adf.json` | **ported** | `assemble-placeholder-collision` (expected.adf.json) — the independent anchor cross-checking the frozen capture |
| `reject-blockquote.md` | represented | `assemble-reject-blockquote` |
| `reject-control-chars.md` | represented | `assemble-reject-control-byte` |
| `reject-jq-injection.md` | dropped | jq string interpolation is a bash-pipeline concern; the Rust `markdown_to_document` serialises text through serde_json, which escapes every text node uniformly (exercised by every `assemble-*` case), so a dedicated differential case adds no coverage the serde path does not already carry |
| `reject-nested-list.md` | represented | `assemble-reject-nested-list` |
| `reject-table.md` | represented | `assemble-reject-table` |
| `underscore-warning.md` | represented | `assemble-notice-underscores` (the `__ __` notice) |
| `underscore-warning.adf.json` | represented | `assemble-notice-underscores` |
| `underscores-as-literals.md` | represented | the `__…__` notice is `assemble-notice-underscores`; the `snake_case`/`_leading_trailing_` literals are ordinary paragraph text carried by `assemble-paragraph` |
| `underscores-as-literals.adf.json` | represented | as above |
| `unsupported-mention.adf.json` | represented | `render-inline-placeholders` (mention → `[unsupported ADF inline: mention]`) |
| `unsupported-panel.adf.json` | represented | `render-block-placeholders` (panel → `[unsupported ADF node: panel]`) |
| `.gitkeep` | dropped | directory placeholder, not a fixture |

**Count**: 43 files — 39 represented, 2 ported (one scenario,
`placeholder-collision`, consuming its `.md` and `.adf.json`), 2 dropped
(`reject-jq-injection.md` and `.gitkeep`). The corpus grows from 56 to **57**
cases; `cli/jira-client/tests/adf_oracle_manifest.rs` pins that count.
