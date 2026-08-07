Bash-golden fixtures for `accelerator-migrate` (work item 0172, Phase 0), captured by `regenerate.sh` driving the still-live bash migration engine (`skills/config/migrate/scripts/run-migrations.sh`) as a black box.

Each fixture directory contains `stdout`, `stderr`, `exit-code`, and `CAPTURE-SOURCE.txt` (the bash-source revision this golden was captured against); most also carry `state/` (the post-run `.accelerator/state/` ledger and manifest files) and/or `result-tree/` (the full post-run `meta/`/`.accelerator/`/`.claude/` tree, for corpus-state byte-parity comparisons). `masks.toml` documents the normalisation rules (sandbox path, timestamps, revision hashes, the `bash run-migrations.sh` → `accelerator migrate` invocation-form substitution) a later comparison harness must apply before diffing against a Rust-produced equivalent — captures themselves are raw, unredacted.

## Fixture matrix

- `all-pending/`, `0001/`…`0006/` — mechanical migrations, reusing the checked-in fixture trees under `skills/config/migrate/scripts/test-fixtures/000N/`.
- `0007/` — real 0007 against a minimal, fully base-field-valid corpus. **Does not exercise the interactive stall path** — see `0007/NOTES.md`.
- `interactive/{doc-example,accept-verb,three-decision,validator-rejecting,foreign-dirty-path,two-owned-dirty-paths}/` — scripted decision sequences and dirty-tree resume/refusal cases, adapted from `test-migrate-interactive.sh`'s own helpers.
- `manifest-states/{absent,empty,unreadable,stale}/` — the four guarded-resume fail-closed states.
- `list/{single-pending,multi-pending}/` — `--list` output shape, including multi-migration `# migration <id>` segmentation.
- `decisions-file/{too-few,too-many,unknown-verb,rejected-edit-no-recovery,blank-crlf-comments}/` — the dry-apply validation pass's fail-closed cases.

## Known gaps / findings for later phases

- **`0007/` doesn't stall.** Constructing a fixture that hits 0007's genuinely `ambiguous`-band interactive prompt needs deeper knowledge of `corpus::linkage::classify_band` than Phase 0 needs to acquire — both a resolvable and an unresolvable body-section reference were tried and both apply mechanically with no decision required. Revisit when Phase 8 starts (see `0007/NOTES.md`).
- **Decisions-file comments are not tolerated by current bash**, contrary to the plan's Phase 5 text. A `#`-prefixed line in a decisions file is rejected as an unknown verb, not skipped — see `decisions-file/blank-crlf-comments/NOTES.md`. If the Rust port is meant to add comment support, that is new behaviour beyond bash parity and should be flagged as such during Phase 5, not silently asserted against this golden.

## Regenerating

`bash regenerate.sh` from this directory (or anywhere — it resolves its own paths). Re-run whenever the bash source it captures changes and the fixtures need refreshing, before that source is deleted in Phase 10.
