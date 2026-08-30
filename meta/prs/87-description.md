---
type: "pr-description"
id: "87"
title: "Canonical frontmatter quoting standard"
date: "2026-08-30T23:17:49+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0221"
parent: "work-item:0221"
relates_to: ["adr:ADR-0065", "work-item:0220", "work-item:0227"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/87"
pr_number: 87
tags: ["frontmatter", "corpus", "document", "emitter", "validator", "migration", "quoting"]
revision: "6e2611e28b35a5353b99a3ae118d3831ef8ea42e"
repository: "accelerator"
last_updated: "2026-08-30T23:17:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Canonical frontmatter quoting standard

## Summary

Make the shared frontmatter renderer emit one canonical, type-driven quoting style — every string double-quoted, integers/booleans/null bare — extend the validator to enforce it, migrate the whole corpus and the `templates/*.md` skeletons into conformance, and add a producer-run validate step to every in-scope writing skill. This ratifies ADR-0065 in code and closes the originating linkage defect (work item 0220), where `work create` emitted unquoted typed-linkage references because the renderer delegated quoting to `serde_saphyr`'s minimal heuristic and nothing ran the validator.

## Changes

Five independently-mergeable phases, plus post-review refinements:

- **Emitter (Phase 1).** The one choke point in `cli/document/src/value.rs` now wraps the string and float scalar arms in `serde_saphyr::DoubleQuoted`; integer/bool/null stay bare. This suppresses the long-value block-scalar refolds (`>-`/`|`) that were the PR #76 churn source. The now-dead tag-item quoting rule in `cli/work/src/tags.rs` is removed — the renderer quotes each element.
- **Migration `m0008` (Phase 2).** A mechanical re-render of every `meta/` file and `.accelerator/config.md` through the canonical emitter (1060 files here), with three fail-closed guards (value-tree comparison, bare-scalar normalisation, in-process structural validation) and a per-file `0008-LOSSY` diagnostic for downstream corpora carrying comments or CRLF. It also realigns the `/sync-work-items` baseline so re-quoting does not spuriously flag every synced item as locally-modified.
- **Validator (Phase 3).** A general `UNQUOTED-STRING` check in a new escape- and quote-aware `canonical_quoting` module flags any bare string on a field the standard requires quoted, while `id`, `schema_version`, timestamps, and typed-linkage keep their dedicated checks. An emitter↔validator symmetry guard pins the two encodings in lockstep.
- **Producer wiring (Phase 4).** 21 in-scope writing skills gained an `allowed-tools` rule and a fenced `corpus frontmatter validate` step on the document they write. A static coverage test over the live `skills/` tree and a CLI signal test pin the enforcement — this is the mechanism in place of a CI conformance lane.
- **Python retirement + template-shape check (Phase 5).** A new Rust `template_shape` check ports the full rule set of the retired `tasks/lint/frontmatter_rules.py` surface; the 13 templates are canonicalised; a `print-schema` subcommand feeds the conformance test its constants from the single Rust definition.

Post-review refinements on this branch: stale planning references (ADRs, ACs, work items) scrubbed from code and template comments; the template-shape guard relegated from a `validate-templates` CLI action to a library-level integration test (the command validated the plugin's own `templates/` dir, meaningless for a plugin user); and the 0201 document family merged from `main` canonicalised to keep the corpus self-check green after the rebase.

## Context

- Work item: `meta/work/0221-canonical-quoting-standard-for-all-frontmatter.md`
- Decision: `meta/decisions/ADR-0065-canonical-frontmatter-quoting-standard.md`
- Plan: `meta/plans/2026-08-30-0221-canonical-frontmatter-quoting-standard.md`
- Validation: `meta/validations/2026-08-30-0221-canonical-frontmatter-quoting-standard-validation.md`
- Originating defect: work item 0220 (unquoted typed-linkage on `work create`)
- Follow-up: work item 0227 (`accelerator config validate`) inherits the shared quoting predicate

## Testing

- [x] Whole corpus validates clean post-rebase: `accelerator corpus frontmatter validate` exits 0, zero violations.
- [x] Build-system unit tests (Phase 4 coverage guard): `mise run test:unit:tasks` — 2740 passed.
- [x] Conformance lane (constants sourced via `print-schema`): `mise run test:integration:conformance` — 27 passed.
- [x] Corpus crates (self-check + template-shape + symmetry guard): 413 passed.
- [x] Migration and work crates (Phases 1/2): 641 passed.
- [x] Clippy and rustfmt across the `cli/` workspace: clean.
- [ ] Full local CI mirror against the rebased base: `mise run` — not re-run since the rebase onto `main`; component lanes above are green.

## Notes for Reviewers

- The diff is dominated by 1060 mechanical `meta/` re-renders (bare → double-quoted, block sequences → quoted flow). The reviewable surface is the ~40 `cli/` code files, the 21 `SKILL.md` edits, and the 13 templates. Focus there.
- `m0008` ships in the plugin binary and runs on every downstream corpus. On a corpus carrying inline frontmatter comments or CRLF it emits a per-file `0008-LOSSY` diagnostic and proceeds; VCS revert is the recovery path.
- Enforcement is producer-run by decision, not a CI lane: a file hand-edited outside a skill is caught only when a skill next rewrites it. AC #6 (static presence of the validate step) and AC #7 (deterministic command signal) are the accepted proxy.
- Two known deferrals, both documented in the plan: change #7 (config-path-aware preflight rescope) hardens `paths.*`-relocated corpora only; and `config validate` (quoting plus semantics) is work item 0227.
- This branch was rebased onto `main` after `main` merged PR #85 (0201). Given that, a full `mise run` before merge is worth it.
