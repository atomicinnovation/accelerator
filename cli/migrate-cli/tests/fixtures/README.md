Bash-golden fixtures for `accelerator-migrate`, captured by `regenerate.sh` driving the bash migration engine (`skills/config/migrate/scripts/run-migrations.sh`) as a black box, before that engine was retired.

Each fixture directory contains `stdout`, `stderr`, `exit-code`, and `CAPTURE-SOURCE.txt` (the bash-source revision this golden was captured against); most also carry `state/` (the post-run `.accelerator/state/` ledger and manifest files) and/or `result-tree/` (the full post-run `meta/`/`.accelerator/`/`.claude/` tree, for corpus-state byte-parity comparisons). `masks.toml` documents the normalisation rules (sandbox path, timestamps, revision hashes, the `bash run-migrations.sh` → `accelerator migrate` invocation-form substitution) a later comparison harness must apply before diffing against a Rust-produced equivalent — captures themselves are raw, unredacted.

## Fixture matrix

- `all-pending/`, `0001/`…`0006/` — mechanical migrations, reusing the fixture trees that were checked in under `skills/config/migrate/scripts/test-fixtures/000N/` before that directory was retired.
- `0007/` — real 0007 against a minimal, fully base-field-valid corpus. **Does not exercise the interactive stall path** (that's covered separately, in `migration_0007.rs`'s own ambiguous-band stall test and in `list_and_decisions_file.rs`) — see `0007/NOTES.md`.
- `interactive/{doc-example,accept-verb,three-decision,validator-rejecting,foreign-dirty-path,two-owned-dirty-paths}/` — scripted decision sequences and dirty-tree resume/refusal cases, adapted from the historical `test-migrate-interactive.sh` test suite's own helpers.
- `manifest-states/{absent,empty,unreadable,stale}/` — the four guarded-resume fail-closed states.
- `list/{single-pending,multi-pending}/` — `--list` output shape, including multi-migration `# migration <id>` segmentation.
- `decisions-file/{too-few,too-many,unknown-verb,rejected-edit-no-recovery,blank-crlf-comments}/` — the dry-apply validation pass's fail-closed cases.

## Findings

- **`0007/` doesn't stall.** Both a resolvable and an unresolvable body-section reference were tried against this fixture's corpus, and both apply mechanically with no decision required — triggering the genuinely `ambiguous`-band interactive prompt needs a reference shape that is structurally ambiguous by construction (per `corpus::linkage::classify_band`), which this fixture's corpus doesn't exercise (see `0007/NOTES.md`).
- **Decisions-file comments are not tolerated.** A `#`-prefixed line in a decisions file is rejected as an unknown verb, not skipped, matching the bash engine's actual behaviour — see `decisions-file/blank-crlf-comments/NOTES.md`.

## Regenerating

`bash regenerate.sh` drove the bash migration engine to produce these fixtures. That engine has since been deleted from the repository, so the script can no longer be re-run; it is kept for provenance.
