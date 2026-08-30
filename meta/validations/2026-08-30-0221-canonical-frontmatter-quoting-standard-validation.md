---
type: "plan-validation"
id: "2026-08-30-0221-canonical-frontmatter-quoting-standard-validation"
title: "Validation Report: Canonical Frontmatter Quoting Standard Implementation Plan"
date: "2026-08-30T22:44:36+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "plan:2026-08-30-0221-canonical-frontmatter-quoting-standard"
target: "plan:2026-08-30-0221-canonical-frontmatter-quoting-standard"
tags: ["frontmatter", "corpus", "document", "emitter", "validator", "migration", "quoting"]
last_updated: "2026-08-30T22:44:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Canonical Frontmatter Quoting Standard

All five phases are implemented, committed, and verified green. One material
deviation post-dates the plan: the `corpus frontmatter validate-templates` CLI
action the plan specifies (Phase 5 change #3, Desired End State) was retired
this session and its guard relegated to a `corpus-adapters` integration test.
That is an intentional improvement, covered by a passing test, but it leaves
the plan's Phase 5 prose stale. Result is **pass**.

### Implementation Status

- ✓ Phase 1 — emitter + tag-rule retirement — fully implemented (`a70f9034`)
- ✓ Phase 2 — `m0008` migration + corpus canonicalisation — fully implemented (`98a4298a`, `5bc07311`, `ab742ecd`)
- ✓ Phase 3 — validator general rule — fully implemented (`be2d9d37`)
- ✓ Phase 4 — producer-skill wiring — fully implemented (`78df7c1b`)
- ✓ Phase 5 — Python retirement + template-shape check — fully implemented (`35b17a43`), with a subsequent refinement (see Deviations)

### Automated Verification Results

Run this session against the current tree (commit `6b6277ac`):

- ✓ Build-system unit tests: `mise run test:unit:tasks` — 2740 passed (includes the Phase 4 producer-coverage guard)
- ✓ Conformance lane: `mise run test:integration:conformance` — 27 passed (sources schema banks via `print-schema`)
- ✓ Corpus crates: `cargo nextest run -p corpus -p corpus-adapters -p accelerator-corpus` — 413 passed (self-check + template-shape + new relocated test)
- ✓ Phase 1/2 crates: `cargo nextest run -p migrate -p accelerator-migrate -p work -p accelerator-work -p document -p config-adapters` — 641 passed
- ✓ Clippy: `mise run lint:cli:check` — clean workspace-wide
- ✓ Rustfmt: `mise run format:cli:check` — clean

Not re-run this session: the single bare `mise run` aggregate. The plan records
it green at completion (`9992fad9`); this session's later commits are covered by
the component lanes above.

### Code Review Findings

#### Matches Plan

- Emitter: `DoubleQuoted` on the string and float arms at `cli/document/src/value.rs:171,175`; integer/bool/null arms bare.
- Dead tag rule: `needs_quoting`/`format_tag` gone from `cli/work/src/tags.rs`; `build_canonical` joins bare.
- Migration: `Migration0008` registered (`registry.rs:78`), `0008-canonical-frontmatter-quoting` in `.accelerator/state/migrations-applied`.
- Validator: `UnquotedString`/`UNQUOTED-STRING` in `violation.rs`; `canonical_quoting.rs` module present; `Checks.canonical` flag wired.
- Producers: 21 in-scope skills carry the fenced validate step; coverage test `test_skill_frontmatter_validation.py` green.
- Template-shape: `template_shape.rs` present; the three Python files deleted; `templates/*.md` carry no bare string fields.
- Validator signal (manual): a bare `author: Toby` exits 1 with `UNQUOTED-STRING`; the fully-quoted form exits 0.

#### Deviations from Plan

- **`validate-templates` CLI action retired (commit `45a94544`, this session).** The plan specifies a `corpus frontmatter validate-templates` subcommand and a `the_real_templates_tree_is_clean` golden that shells out to it (Phase 5 change #3; Desired End State line 137; change #1 lines 860-866). The subcommand only ever validated `<project-root>/templates/`, which is the plugin's own tree — for any plugin user it validates a non-existent directory and never resolves user template overrides. It is now a library-level `corpus-adapters` integration test (`the_shipped_templates_tree_is_clean`) calling `validate_templates` directly. The pure `template_shape` logic and the guard are unchanged; only the misleading command surface was removed. Plan prose at lines 137, 860-866, 918, 967 is now stale.
- **Frontmatter-comment scrub (commit `0c895f20`, this session).** Removed stale ADR/AC/work-item references from code and template comments per the project comment policy; not a plan item, orthogonal to the standard.
- Per-phase deviations already recorded inline in the plan and re-confirmed here: `canonicalise_frontmatter` port dropped (m0008 calls `document::render` directly); `Checks.canonical` added for migration 0007's pre-canonical self-check; template-shape reads per-type facts from the TSV rows, not `schema.rs::SCHEMA`; `print-schema` emits hand-rolled JSON; the coverage test binds to a fenced `bash` block rather than a heading regex.

#### Potential Issues

- The plan is already `status: done` but its Phase 5 text still describes the retired CLI action. A reader following the plan would look for a subcommand that no longer exists. Recommend a short amendment note in the plan pointing at this deviation (the plan is a historical artefact, so a superseding note rather than a rewrite).
- Change #7 (config-path-aware preflight rescope) is deferred, as the plan records. Repos on default paths are unaffected; a `paths.*`-relocated corpus keeps the pre-existing dirty-tree blind spot. Tracked, not a regression from this work.

### Manual Testing Required

Covered by automated tests but not exercised live this session:

1. Emitter round-trip:
  - [ ] `accelerator work create "T" bug medium --parent "work-item:0171" --relates-to "work-item:0194"` on a scratch repo writes fully double-quoted linkage/tags with no `>-` refolds (covered by `document`/`work` tests, 641 green).
2. Sync baseline:
  - [ ] After migrating a repo carrying `last-sync.json`, `/sync-work-items` preview reports no spurious locally-modified items (covered by the both-directions realignment tests).
3. Producer skills:
  - [ ] Run `create-work-item` and `create-note` end-to-end and confirm the validate step surfaces a violation on a non-conformant write (the CLI signal it invokes is verified automatically).

### Recommendations

- Add a one-line superseding note to the plan's Phase 5 recording that the `validate-templates` CLI action was retired in favour of a library-level test, so the plan text and the tree agree.
- Leave change #7 as tracked follow-up work; it is downstream-only and out of this plan's committed scope.
- No blocking issues — the standard is realised end-to-end and every automated lane run this session is green.
