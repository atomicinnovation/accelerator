---
type: inventory
id: "0167-divergences"
title: "0167 Recorded Divergences from the Bash Config Cluster"
date: "2026-07-21T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: complete
parent: "work-item:0167"
relates_to:
  - "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
tags: [rust, config, cli, migration, divergences]
last_updated: "2026-07-21T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0167: Recorded Divergences

Each row is a deliberate departure from the bash config cluster's observable
behaviour, with the test that would fail if the divergence were absent or later
regressed. A divergence nothing can detect is indistinguishable from a defect,
so every row names a real, passing test.

Test locations:

- `read.rs` — `cli/launcher/tests/config_read.rs` (black-box)
- `parity.rs` — `cli/config-adapters/tests/parity.rs`
- `store.rs` — `cli/config-adapters/src/store.rs` unit tests
- `compose.rs` — `cli/config-adapters/src/compose.rs` unit tests

| # | Divergence | Pinning test | Status |
|---|---|---|---|
| 1 | `assert_no_legacy_layout` applied uniformly; `--allow-legacy-layout` restores both the guard suppression and the legacy source fallback, on reads only | `read.rs::a_legacy_layout_is_refused_by_a_read`, `read.rs::allow_legacy_layout_suppresses_the_refusal_and_reads_the_legacy_pair`, `read.rs::the_legacy_fallback_is_inert_when_the_current_pair_is_present`, `compose.rs::fails_closed_on_the_legacy_layout` | done |
| 2 | The `config-read-review.sh:270` double slash in a custom-lens path is fixed to a single slash | `read.rs::a_custom_lens_row_uses_a_single_slash_path_and_the_right_source` | done |
| 3 | The init-sentinel resolves against the project root, not the caller's CWD | `read.rs::summary_resolves_the_init_sentinel_against_the_project_root` | done |
| 4 | Usage errors exit 1 (not clap's 2), so exit 2 is reserved for a subcommand refusal | `read.rs::a_usage_error_exits_one_not_two` | done |
| 5 | Fail-safe is an explicit `--fail-safe` opt-in that degrades read/IO failures only; validation refusals stay fail-closed | `read.rs::work_stays_fail_closed_on_a_bad_enum_even_with_fail_safe`, `read.rs::paths_doc_types_stays_fail_closed_on_escape_with_fail_safe`, `read.rs::a_scalar_with_fail_safe_suppresses_a_read_failure_and_exits_zero` | done |
| 6 | The `## <Name> Unavailable` fail-open blocks are net-new output | `read.rs::agents_with_fail_safe_renders_the_unavailable_notice`, `read.rs::dump_with_fail_safe_renders_the_unavailable_notice`, `read.rs::review_with_fail_safe_renders_the_unavailable_notice` | done |
| 7 | `paths --doc-types` buffers all 13 rows and emits only on success (bash leaves a partial prefix) | `read.rs::paths_doc_types_refuses_an_unsafe_path_with_empty_stdout` | done |
| 8 | Skill and template names are validated as identifiers before interpolation | `read.rs::an_invalid_skill_name_without_fail_safe_exits_non_zero`, `read.rs::a_traversing_template_name_is_refused` | done |
| 9 | The inner `.gitignore` is ensured, not created-if-absent | `store.rs::a_personal_write_ensures_the_inner_gitignore` | done (Phase 1) |
| 10 | Malformed frontmatter is fail-loud where the bash degrades | `parity.rs::malformed_frontmatter_is_fail_loud_where_bash_degrades` | done (0178) |
| 11 | A block-authored YAML sequence resolves to a populated sequence | `parity.rs::a_block_authored_array_diverges_from_the_bash_found_empty` | done (0178) |
| 12 | Divergent value encodings resolve to their declared forms | `parity.rs::value_encodings_resolve_to_their_declared_divergent_forms` | done (0178) |
| 13 | `context --skill` inserts one blank line between the project and skill blocks | `read.rs::context_skill_joins_both_blocks_with_one_blank_line` | done |
| 14 | `init` sources its 14 directories from `catalogue::default_for` | `read.rs::init_creates_the_documented_tree` (plus the `core::init::tests::each_dir_default_matches_the_catalogue` unit test pinning the coincidence) | done (Phase 3) |
| 15 | Recoverable stderr warnings carry a uniform `Warning: ` prefix, not the bash per-script `config-read-*.sh: warning:` / bare form | `test-config.sh` (repointed): the `config review`/`path`/`work`/`template` warning greps | done (Phase 4) |
| 16 | An invalid `review` mode is reported via clap's value-enum error (`[possible values: pr, plan, work-item]`), not the bash `<pr\|plan\|work-item>` usage line | `test-config.sh` (repointed): "unknown mode -> exit 1 and usage contains …" | done (Phase 4) |

## Notes

- Divergence 5 is the load-bearing read/IO-vs-validation split: a
  `Failure::Read` degrades under `--fail-safe`; a `Failure::Refusal` (a
  `ConfigError::Invalid` — bad `work.integration` enum, doc-type escape,
  unresolvable/invalid template name) stays fail-closed regardless.
- The `## <Name> Unavailable` notice headers are exactly: `## Agent Names
  Unavailable`, `## Project Context Unavailable`, `## Skill-Specific Context
  Unavailable`, `## Skill Instructions Unavailable`, `## Review Configuration
  Unavailable`, plus the multi-line-block notices `## Configured Paths
  Unavailable`, `## Effective Configuration Unavailable`, `## Template
  Unavailable`.
