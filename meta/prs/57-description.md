---
type: "pr-description"
id: "57"
title: "[0195] accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI"
date: "2026-08-08T14:52:17+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0195"
parent: "work-item:0195"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/57"
pr_number: 57
tags: []
revision: "3d6eebd9ddd5b7f0fe63ee72f445a2115f46b496"
repository: "accelerator"
last_updated: "2026-08-08T14:52:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0195] accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI

## Summary

Builds `accelerator-corpus`, a new dispatched sub-binary exposing `accelerator corpus <noun> <verb>` over four noun groups — `adr`, `metadata`, `linkage`, `frontmatter` — replacing five bash entry points (`adr-next-number.sh`, `adr-read-status.sh`, `artifact-derive-metadata.sh`, `validate-corpus-frontmatter.sh`, `linkage-parser.sh`) and every caller of them across skills and the 0007 config migration script. Delivered as five independently mergeable phases plus follow-up fixes, then validated end to end.

## Changes

- New `cli/corpus-cli/` crate (`accelerator-corpus`) registered as a dispatched sub-binary, following the `accelerator-vcs`/`accelerator-work` precedent (`tasks/shared/paths.py`, `tasks/manifest.py`, `tasks/build.py`, `.gitignore`, `tests/integration/tasks/test_github.py`'s upload-count derivation).
- `adr next-number` / `adr read-status` — ports the ADR numbering/status-scanning state machine into `corpus::adr`, preserving bash quirks deliberately (untruncated width past 4 digits, the three-stage quote/whitespace stripping order, fence-close-breaks-immediately semantics).
- `metadata derive` — thin CLI glue over the already-shipped, differentially parity-tested `corpus_adapters::metadata::{derive_at, render}`.
- `linkage extract` — thin CLI glue plus new config→doc-type-table wiring (`table_from_config`) over the already-shipped, differentially parity-tested `corpus::linkage::parse_document`.
- `frontmatter validate` — new domain logic: a 13-row per-type schema table, 17 violation codes (16 ported from bash plus a new `DuplicateId` referential-integrity check bash's associative-array index couldn't detect), and a whole-corpus walk/index with default structure+references checking. Ports bash's raw-text `parse_entries` scanner (not just the parsed YAML mapping) because quote-syntax information (`UNQUOTED-ID`, `BAD-LINKAGE-SHAPE`) doesn't survive a real YAML parse.
- Rewrites every caller: 15 `SKILL.md` call sites (with `allowed-tools` grants scoped per skill to the verbs it actually invokes) and the 0007 migration script's two validator/linkage call-outs, now wrapped in the same `log_warn`/VCS-revert guard pattern every other failure branch in that script uses.
- Deletes the five source scripts plus their dedicated bash test harnesses (`test-adr-scripts.sh`, `test-linkage-parser.sh`, `test-validate-corpus-frontmatter.sh` — the latter a `_REQUIRED_CONFIG_SUITES` CI gate, replaced by an unconditional whole-corpus `frontmatter validate` self-check as the new fail-closed guarantee). Adjusts the decisions/config suite-count floors accordingly.
- Adds `docs-site/src/content/docs/corpus.md` and a README "Corpus CLI" entry; adds `ACCELERATOR_CORPUS_BIN` dev-override wiring mirroring `ACCELERATOR_VCS_BIN`.
- Four post-implementation fixes: corrected trailing-quote stripping order in `adr read-status`, refused `paths.*` lookups for keys the catalogue doesn't recognise, moved ADR-number fetching in `create-adr` to happen live rather than at skill load, and stripped now-dead bash-script references from comments.
- Validated the plan (`meta/validations/2026-08-06-0195-...-validation.md`, result: pass) and marked its status `done`; repointed five `templates/*.md` placeholder comments off the now-deleted `artifact-derive-metadata.sh` onto `accelerator corpus metadata derive`.

## Context

- Work item: `meta/work/0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md` (status: done)
- Plan: `meta/plans/2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md`
- Research: `meta/research/codebase/2026-08-06-0195-accelerator-corpus-cli-implementation-surface.md`
- Validation: `meta/validations/2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli-validation.md`

## Testing

- [x] `cargo test --manifest-path cli/Cargo.toml -p accelerator-corpus -p corpus -p corpus-adapters` — all suites pass, including the unconditional whole-corpus `frontmatter validate` self-check.
- [x] `mise run check` — full read-only CI mirror (format/lint/types across frontend, server, cli, build-system, scripts) passes.
- [x] `mise run cli:check`, `mise run lint:dispatch-coherence:check` — pass.
- [x] `mise run test:integration:config` (floor 16, `_REQUIRED_CONFIG_SUITES` down to one entry) and `mise run test:integration:decisions` (floor 0) — pass.
- [x] `uv run pytest tests/integration/tasks/test_github.py tests/unit/tasks/shared/test_dispatch_coherence.py tests/unit/tasks/test_signing.py tests/integration/tasks/test_release.py` — 154 passed.
- [x] Manual smoke test of all four subcommands (`adr next-number`, `adr read-status`, `metadata derive`, `linkage extract`, `frontmatter validate`) against real files in this repository.
- [x] Repo-wide grep confirms zero remaining executable invocations of the five removed scripts outside documentation/ADR prose.
- [ ] Interactive re-invocation of the rewritten skills (`/accelerator:create-adr`, `/accelerator:create-plan`, etc.) to confirm no stale-`allowed-tools` permission prompt fires — not exercised in this session (requires an interactive Claude Code session with this branch's binary on the dispatch path); noted as open in the plan and validation report.

## Notes for Reviewers

- Phase 4 (`frontmatter validate`) is the largest and most architecturally novel piece — see its "Design correction found during implementation" note in the plan for why it validates raw frontmatter text rather than only the parsed YAML mapping.
- Two genuine pre-existing corpus frontmatter issues were fixed as part of reaching a clean self-check baseline (visible in this diff's `meta/` changes) — flagged in the plan, not new scope creep.
- The 0007 migration's `self_validate_structural`/`self_validate_referential` call sites now run referential-integrity checking by default where bash's file-list mode never did — an intentional widening, called out explicitly in the plan rather than a silent behavioural change.
