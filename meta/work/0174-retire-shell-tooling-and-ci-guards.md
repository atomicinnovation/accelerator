---
type: work-item
id: "0174"
title: "Empty scripts/ and Retire Shell Tooling and CI Guards"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: ready
kind: story
priority: medium
parent: "work-item:0136"
blocked_by: []
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [shell, tooling, ci, cleanup]
last_updated: "2026-08-28T00:05:44+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-195"
---

# 0174: Empty scripts/ and Retire Shell Tooling and CI Guards

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

So build-system maintainers no longer carry guards that exist only to police a
shell layer that is gone: as each script cluster's last shell file disappears,
retire the build-system and CI machinery that exists only to police the vanished
shell library — the exec-bit invariant, `SHELL_LIBRARIES`, the shell-suite floors,
`shell_sources()`, and the `check-scripts` release-gate job — shrinking the shell
surface toward the thin-wrapper floor ADR-0048 targets. The bashisms denylist is
re-homed rather than dropped: its shell implementation (`scripts/lint-bashisms.sh`)
is reimplemented as a Python task under `tasks/`, so no shell tool lints shell, and
shfmt + ShellCheck are retained — all three rescoped from the whole-tree walk to the
handful of surviving thin-shell files (the launcher bootstrap, hook wrapper and
Playwright executor). Beyond the guards, empty the
`scripts/` directory itself: delete the residual shell libraries, their `test-*.sh`
harnesses and orphaned fixtures; relocate the two data files that Rust consumes as
production or drift-test source-of-truth into their consuming `cli/` crate; and port
the nine authoring and evals guards that have no Rust equivalent to the Python
build-system.

## Context

A substantial build-system layer exists purely to guard the bash library: the
bashisms denylist (ADR-0049 floor), the exec-bit invariant + `SHELL_LIBRARIES`
frozenset, the shell-suite discovery + minimum-count floors, shfmt + ShellCheck, the
`.shellcheckrc` and `[*.sh]` editorconfig block, and the `check-scripts` CI job (a
release gate). Most become removable outright as the scripts they police disappear;
the bash-3.2 guard is the exception — a thin slice of shell remains (the launcher
bootstrap, the hook wrapper, the Playwright executor) and stays under the bash-3.2
floor, so the bashisms denylist is re-homed to Python and shfmt + ShellCheck are
retained, all rescoped to that survivor set rather than removed. The floors must be
decremented in lockstep with suite retirement to avoid a green→red CI gap.

With every domain cluster now migrated to the `cli/` Rust workspace (0167–0172,
0195–0197, 0211, 0212, all done), the `scripts/` directory has become almost entirely
residue: shell libraries whose behaviour lives in `cli/`, `test-*.sh` harnesses that
either duplicate Rust tests or lint authored content, and a few data files still read
by Rust. A 2026-08-27 audit classified all forty-nine remaining files into five
dispositions — delete outright, delete after a single live-coupling cutover, relocate
into `cli/`, rescope, or port to Python — recorded in the disposition table under
Technical Notes. The migration is behaviourally complete; what blocks a clean `scripts/` is a
small set of live couplings, not missing Rust logic. The most load-bearing is a single
bash call in `skills/integrations/jira/create-jira-issue/SKILL.md`, which keeps the
entire config source-chain alive until it is repointed to a Rust `external_id`
writeback.

## Requirements

- Decrement each shell-suite floor in `tasks/test/integration.py` and shrink
  `SHELL_LIBRARIES` in `tasks/lint/scripts.py` in the same change that deletes the
  corresponding scripts — never leaving a floor expecting a removed suite. The four
  floors this story owns outright — config, hooks, decisions and github — are brought
  to zero and removed within this story, not deferred to any other (config and hooks
  are decremented as their suites go; decisions and github already stand at 0); 0171
  owns the work and integrations pair (see Technical Notes).
- Once a checker has no remaining inputs, remove it: the exec-bit invariant guard
  and the `SHELL_LIBRARIES` frozenset. Delete the shell bashisms linter
  `scripts/lint-bashisms.sh` but reimplement its denylist as a Python task under
  `tasks/`, so no shell tool lints shell.
- Retain shfmt, ShellCheck, the `.shellcheckrc` and the `[*.sh]` editorconfig block
  to guard the surviving thin shell, rescoping them (and the new Python bashisms task)
  from the `shell_sources()` whole-tree walk to an explicit list of the surviving
  files.
- Remove `shell_sources()` (`tasks/shared/sources.py`) and the `check-scripts` CI
  job (`.github/workflows/main.yml`), enumerating every `needs: check-scripts` edge —
  the release gate and any other job — and repointing or dropping each, once no
  policed shell remains.
- Keep the surviving thin shell (launcher bootstrap, hook wrapper, Playwright
  executor) bash-3.2-safe, guarded automatically by the Python bashisms task plus
  shfmt and ShellCheck over the explicit surviving-file list — not by hand review.
- Do **not** carry the work-item or integration clusters. 0171 was widened on
  2026-08-17 to delete every `work-item-*.sh` and `test-work-item-*.sh` outright
  — including `work-item-sync-label.sh`, `work-item-normalise.sh` and
  `work-item-file-dirty.sh`, which earlier drafts of both stories deferred here —
  and to migrate the jira and linear integration scripts. It therefore owns the
  removal of `_EXPECTED_WORK_SUITES` (floor and `_require_suite_floor` call
  alike, not a decrement), the `_EXPECTED_INTEGRATIONS_SUITES` retirement, and
  eight of the twenty-two `SHELL_LIBRARIES` entries: the
  `skills/work/scripts/work-item-bridge-codes.sh` entry plus all seven jira and
  linear library entries. This story's lockstep obligation covers only what
  remains — the fourteen `scripts/*.sh` entries and the config, hooks, decisions
  and github floors.
- Sever the one live bash coupling first: repoint
  `skills/integrations/jira/create-jira-issue/SKILL.md` off
  `config_upsert_frontmatter_field` to the Rust `external_id` writeback
  (`cli/work-cli/src/sync_author.rs`), then delete `config-common.sh` together with
  its `vcs-common.sh`, `config-defaults.sh` and `atomic-common.sh` source chain and
  the drift-oracle tests — each a Rust test that diffs the crate against the bash
  file to catch the two drifting apart — that read them
  (`cli/config/tests/extra_keys_mirror.rs`, the `config-defaults.sh` reads in the
  corpus parity suites).
- Delete the orphaned libraries and their paired tests outright — `log-common.sh`,
  `accelerator-scaffold.sh`, `doc-type-table.sh`, `hash-common.sh`+`test-hash-common.sh`,
  `fs-common.sh`+`test-merge-move.sh` — plus `status-legacy-map.tsv` and the
  `test-fixtures/config-read-review/` goldens (their production script was removed
  in 0167).
- Delete `doc-type-inference.sh` and `test-doc-type-inference.sh` once
  `cli/corpus-adapters/tests/parity.rs` — the parity test that sources it as a bash
  oracle — is removed; `corpus::doc_type::infer` is already natively unit-tested.
- Rescope `cli/corpus-adapters/tests/doc_type_single_source.rs` rather than delete
  it: of its three cases, only `every_non_virtual_type_is_registered_exactly_once`
  is bash-free — it cross-checks the compiled `config paths --doc-types` resolver
  against `DocTypeKey`, so it survives and is lifted out of the `bash-parity`
  feature gate. Drop the other two, which are pure bash-data oracles:
  `every_config_path_key_exists_in_the_config_schema` (sources `config-defaults.sh`
  `PATH_KEYS`) and `the_type_pair_table_matches_the_tsv` (reads
  `linkage-type-pairs.tsv`).
- Relocate the two data files Rust consumes — `templates-schema.tsv`
  (`include_str!` into `cli/corpus/src/frontmatter_validation/schema.rs`) and
  `extract-work-items-cue-phrases.txt` (`cli/design/tests/cue_phrase_drift.rs`) —
  into their consuming crate and repoint the `include_str!`/`require_file` paths;
  their removal from `scripts/` must not break the corpus binary or its drift tests.
  `linkage-type-pairs.tsv` is **not** relocated: `corpus::linkage::TYPE_PAIRS` is a
  hand-maintained const that only mirrors it, and once `the_type_pair_table_matches_the_tsv`
  is dropped nothing else reads the file — so it is deleted outright with the bash
  scripts.
- Port the nine authoring/evals guards with no Rust equivalent to Python under
  `tests/` + `tasks/` — `test-format`, `test-hierarchy-format`, `test-lens-structure`,
  `test-boundary-evals`, `test-evals-structure`, `test-evals-structure-self`,
  `test-skill-frontmatter-conformance`, `test-skill-frontmatter-population`,
  `test-template-frontmatter` — as **eight** standalone pytest guards:
  `test-evals-structure-self` is a meta-test asserting `test-evals-structure` and
  `test-hierarchy-format` classify their fixtures correctly, so its assertions fold
  into those two ported guards' fixture cases rather than becoming a ninth standalone
  case. Carry `skills-schema.tsv` and the frontmatter rules/fixtures as pytest
  fixtures. The five domain-logic tests (`atomic`, `hash`, `vcs`,
  `doc-type-inference`, `merge-move`) are already mirror-tested in `cli/` and are
  deleted without a port.
- Drop the retired-script references from `tasks/measure.py` (the `RECOVERED_FILES`
  `vcs-common.sh` pin) and `tasks/lint/call_site_migration.py` (the `config-common.sh`,
  `doc-type-inference.sh`, `doc-type-table.sh` allowlist entries) in the same change
  that deletes each file, and leave no dangling reference in `skills/`, `hooks/`,
  `.github/` or `.editorconfig`.

## Acceptance Criteria

- [ ] Each suite-floor decrement and `SHELL_LIBRARIES` shrink lands in the same
      change that deletes the corresponding scripts, and that change is independently
      verified to exit 0 on `mise run` before the next lands — so CI never goes
      green→red on a floor mismatch.
- [ ] The exec-bit invariant, `SHELL_LIBRARIES` frozenset, and shell
      `lint-bashisms.sh` are removed; the bashisms denylist runs as a Python `tasks/`
      task instead; shfmt, ShellCheck, `.shellcheckrc` and the `[*.sh]` editorconfig
      block are retained but rescoped to the surviving thin-shell files; `mise run
      check` / bare `mise run` still pass.
- [ ] `shell_sources()` and the `check-scripts` CI job are removed; every
      `needs: check-scripts` edge in `.github/workflows/main.yml` — the release gate
      and any other job — is enumerated and repointed or dropped, so the CI job graph
      has no dangling dependency.
- [ ] The repointed `create-jira-issue/SKILL.md` writes the `external_id` frontmatter
      field equivalently to the retired `config_upsert_frontmatter_field` upsert — for
      both the insert case (no prior `external_id`) and the update case (existing
      `external_id` overwritten), the written value matches the retired path. Verified
      against the local writeback path only (no live Jira API call), so the check
      carries no external-service coupling, before `config-common.sh` and its source
      chain are deleted.
- [ ] The surviving thin-shell files are enumerated in `tasks/README.md` with their
      bash-3.2 constraint recorded (ADR-0049).
- [ ] The surviving thin shell's bash-3.2 safety is verified automatically by the
      Python bashisms task plus shfmt and ShellCheck, all exiting 0 over the explicit
      surviving-file list — and the list those guards actually scan is asserted equal
      to the `tasks/README.md` enumeration (a single source of truth), so a dropped
      survivor is caught.
- [ ] `scripts/` retains no shell library, `test-*.sh`, orphaned fixture, or bashisms
      linter — the denylist is now a Python `tasks/` task — so `find scripts -name
      '*.sh'` returns nothing. Every surviving thin-shell file (launcher bootstrap,
      hook wrapper, Playwright executor) is homed outside `scripts/` (in `hooks/` and
      `cli/`), and that full set matches the `tasks/README.md` enumeration exactly.
- [ ] No dangling reference to any deleted `scripts/` file remains in `skills/`,
      `hooks/`, `tasks/`, `cli/`, `.github/` or `.editorconfig`; a repo-wide grep for
      each removed path resolves only to surviving or relocated locations.
- [ ] Before each shell guard is deleted, its violating inputs are captured as a
      named golden fixture set (reusing the predecessor's existing shell fixtures
      where present); each of the eight standalone ported pytest guards then fails on
      every fixture in its set and passes on the conforming ones — with the
      `-self` meta-test's assertions verified through the two guards it folds into —
      so parity is verified against an enumerated corpus rather than an assertion of
      sameness.
- [ ] The re-homed Python bashisms task is fixture-verified to the same standard as
      the ported guards: on a captured golden corpus it flags every bash-4 construct
      the retired `lint-bashisms.sh` caught (associative arrays, `${var,,}`, and the
      rest) and passes the conforming survivors; and shfmt and ShellCheck each fail on
      a deliberately malformed copy of a surviving file — so the retained bash-3.2
      guards are proven able to fail, not merely to exit 0.
- [ ] The two relocated data files (`templates-schema.tsv`,
      `extract-work-items-cue-phrases.txt`) live under `cli/` and their consuming
      `include_str!`/`require_file` tests pass; `linkage-type-pairs.tsv` is deleted
      with the bash scripts; `doc_type_single_source.rs` retains only
      `every_non_virtual_type_is_registered_exactly_once` (de-gated and passing), and
      every other drift-oracle test that read deleted shell is removed.
- [ ] `mise run` (bare default) exits 0 end-to-end; every commit that removes scripts
      or decrements a floor is independently verified green before the next — no
      green→red gap at any step.

## Open Questions

- **Resolved 2026-08-28** — the bashisms/thin-shell question: the denylist is
  reimplemented as a Python task under `tasks/` and shfmt + ShellCheck are retained,
  all rescoped to the surviving thin-shell files (see Requirements). No open question
  now gates promotion.
- The data-file relocation form (does not gate promotion): whether the two relocated
  data files (`templates-schema.tsv`, `extract-work-items-cue-phrases.txt`) are
  inlined as Rust constants or moved as crate resource files still read via
  `include_str!`/`require_file`. An implementation detail settled during the work.

## Dependencies

- Blocked by: none remaining. All predecessors — 0167, 0168, 0169, 0170,
  0171, 0172, 0195, 0196, 0197, 0211, 0212 — reached `done` as of
  2026-08-28, so this story is now unblocked.
- The Rust `external_id` writeback at `cli/work-cli/src/sync_author.rs:145-159`
  that the Jira live-coupling cutover repoints onto was delivered by the Jira
  integration migration in 0211/0212 (both done), so the cutover's upstream is a
  completed, verifiable item rather than an inferred one.
- Parent: epic 0136.

## Assumptions

- A residual thin shell surface remains (bootstrap, hook wrapper, Playwright
  executor); full removal of all shell is not the goal (ADR-0048 thin-wrapper
  floor).

## Technical Notes

- Anchors: `scripts/lint-bashisms.sh`; `tasks/lint/scripts.py:18,86,100`
  (SHELL_LIBRARIES, bashisms, exec-bits); `tasks/test/integration.py:8-36` (floors);
  `tasks/format/scripts.py:9` (shfmt); `tasks/lint/scripts.py:70` (shellcheck);
  `tasks/shared/sources.py:60` (`shell_sources`); `.shellcheckrc`;
  `.editorconfig:36-39`; `.github/workflows/main.yml:99` (`check-scripts`).
  Post-resolution disposition of these anchors: **remove** SHELL_LIBRARIES, exec-bits,
  floors, `shell_sources()` and `check-scripts`; **re-home** `lint-bashisms.sh` (:86)
  as a Python task; **retain and rescope** shfmt (:9), ShellCheck (:70), `.shellcheckrc`
  and the `[*.sh]` editorconfig block to the explicit thin-shell survivor list rather
  than the `shell_sources()` walk.
- Floor ownership as of 2026-08-17. `tasks/test/integration.py` declares six:
  `_EXPECTED_CONFIG_SUITES = 15` (:45), `_EXPECTED_WORK_SUITES = 5` (:51),
  `_EXPECTED_INTEGRATIONS_SUITES = 32` (:57), `_EXPECTED_HOOKS_SUITES = 1` (:76),
  `_EXPECTED_DECISIONS_SUITES = 0` (:77) and `_EXPECTED_GITHUB_SUITES = 0` (:78).
  0171 removes the work and integrations pair; this story owns the other four —
  `_EXPECTED_CONFIG_SUITES` (15) and `_EXPECTED_HOOKS_SUITES` (1) are decremented to
  zero as their suites go and then removed, while `_EXPECTED_DECISIONS_SUITES` and
  `_EXPECTED_GITHUB_SUITES` already stand at 0 and are simply removed. No config- or
  hooks-cluster story performs those decrements.
- `SHELL_LIBRARIES` ownership: 0171 clears the single `skills/work/scripts/`
  entry and the seven `skills/integrations/{jira,linear}/scripts/` entries. The
  fourteen `scripts/*.sh` entries — `fs-common`, `hash-common`, `log-common`,
  `work-common`, `config-defaults`, `config-common`, `atomic-common`,
  `vcs-common`, `doc-type-table`, `doc-type-inference`,
  `frontmatter-emission-rules`, `frontmatter-fixtures`, `test-helpers` and
  `accelerator-scaffold` — are removed by this story directly (every owning cluster is
  already done), each dropped from the frozenset in the change that deletes its file;
  the frozenset itself disappears once the last is gone.

### `scripts/` disposition (2026-08-27 audit)

Every remaining `scripts/` file falls into one of five dispositions. The one
non-`scripts/` entry — `doc_type_single_source.rs` in the Rescope row — is a
downstream `cli/` consumer the audit tracks because its couplings gate a `scripts/`
deletion; it is not one of the forty-nine `scripts/` files.

| Disposition | Files | Basis |
|---|---|---|
| Delete outright | `log-common.sh` (no consumer at all), `accelerator-scaffold.sh`, `doc-type-table.sh`, `hash-common.sh`+`test-hash-common.sh`, `fs-common.sh`+`test-merge-move.sh`, `status-legacy-map.tsv`, `linkage-type-pairs.tsv`, `test-fixtures/config-read-review/` | Orphaned; behaviour in `cli/` store/migrate-adapters/corpus. `linkage-type-pairs.tsv` has no production consumer once its one drift test is dropped |
| Delete after cutover | `config-common.sh`→`{vcs-common, config-defaults, atomic-common}` + `test-atomic-common.sh`/`test-vcs-common.sh`; `doc-type-inference.sh`+`test-doc-type-inference.sh` | Single live coupling each — see below |
| Relocate into `cli/` | `templates-schema.tsv`, `extract-work-items-cue-phrases.txt` | Read by Rust via `include_str!` (`templates-schema.tsv` production; `cue-phrases` a drift test) |
| Rescope (keep valuable case) | `doc_type_single_source.rs` | Keep the bash-free resolver-vs-`DocTypeKey` case; drop its two bash-data oracles |
| Port to Python | the nine authoring/evals guards + `skills-schema.tsv`, `frontmatter-emission-rules.sh`, `frontmatter-fixtures.sh` as fixtures; `lint-bashisms.sh` (denylist → `tasks/` task) | Lint authored content / shell, no Rust equivalent; the bashisms port keeps an automated bash-3.2 guard without shell linting shell |

Live-coupling anchors (sever before the corresponding delete):

- `skills/integrations/jira/create-jira-issue/SKILL.md:111-112` — sources
  `config-common.sh`, calls `config_upsert_frontmatter_field`; the sole live entry
  into the config chain. Rust successor: `cli/work-cli/src/sync_author.rs:145-159`.
- `cli/corpus-adapters/tests/parity.rs:94,111-136` — runs `doc-type-inference.sh`
  as a bash oracle; deleted with the script.
- `cli/corpus-adapters/tests/doc_type_single_source.rs` — sources `config-defaults.sh`
  `PATH_KEYS` (:64) and `require_file`s `linkage-type-pairs.tsv` (:111) in its two
  bash oracles (dropped); its `every_non_virtual_type_is_registered_exactly_once`
  case (:26-53) runs the compiled `config paths` resolver, is bash-free, and is
  retained (de-gated from `feature = "bash-parity"`).
- `cli/config/tests/extra_keys_mirror.rs:14` — reads `config-defaults.sh` as a drift
  oracle.
- `cli/corpus/src/frontmatter_validation/schema.rs:277` — `include_str!` on
  `templates-schema.tsv` (production, baked into the corpus binary).
- `cli/design/tests/cue_phrase_drift.rs:11` — `include_str!` on
  `extract-work-items-cue-phrases.txt`.
- `tasks/measure.py:982` (`RECOVERED_FILES` `vcs-common.sh` pin) and
  `tasks/lint/call_site_migration.py` (allowlist for `config-common.sh`,
  `doc-type-inference.sh`, `doc-type-table.sh`).

### Where lost test coverage lands: Python, not Rust

The fourteen `test-*.sh` split cleanly. The five exercising migrated domain logic —
`test-atomic-common`, `test-hash-common`, `test-vcs-common`, `test-doc-type-inference`,
`test-merge-move` — are already mirror-tested in `cli/` (`store`, `corpus-adapters`
lock/JSONL, `vcs-adapters` classify/detection, `corpus::doc_type`, `migrate-adapters`
merge_move), so they are deleted without replacement. The nine survivors — ported as
eight standalone pytest guards, `test-evals-structure-self` folding in —
`test-format`, `test-hierarchy-format`, `test-lens-structure`, `test-boundary-evals`,
`test-evals-structure`, `test-evals-structure-self`,
`test-skill-frontmatter-conformance`, `test-skill-frontmatter-population`,
`test-template-frontmatter` — lint the plugin's own authored content (SKILL.md bodies,
`templates/*.md`, review-lens structure, evals JSON pairing and pass-rate floors).
That is a build-system/authoring concern with no migrated domain logic behind it, and
the build system is Python (pytest under `tests/`, invoke tasks under `tasks/`) — so
their replacements belong in the Python suites, not the Rust ones. The Rust corpus
tests cover only the validator engine on synthetic documents; they never assert the
shipped templates' shape or the SKILL.md-literal conformance these guards enforce.

## Drafting Notes

- Treated as the Phase 11 cross-cutting cleanup story; much of its work lands
  incrementally inside the subdomain stories (lockstep floor decrements), with the
  final checker/CI-job removals gated on all clusters retiring — hence the broad
  `blocked_by`.
- Updated 2026-08-17: scope moved out to 0171. Earlier drafts of 0171 retained
  `work-item-sync-label.sh` and `work-item-normalise.sh` and left
  `work-item-file-dirty.sh` undecided, all three landing here as residue; 0171
  now deletes the whole `work-item-*.sh` surface itself. This story's generic
  lockstep language did not name the residue, so nothing here was wrong — but it
  did leave ownership of the work and integrations floors and eight
  `SHELL_LIBRARIES` entries ambiguous between the two items. Both are now stated
  explicitly in Requirements and Technical Notes, so a floor cannot be
  decremented twice or missed entirely.
- Updated 2026-08-28: every predecessor reached `done`, so `blocked_by` was
  cleared and the Dependencies prose reconciled. The earlier prose/frontmatter
  divergence (prose named 0171; frontmatter listed 0211/0212 instead of 0171)
  is now moot — all are complete either way, and the full set is recorded above.
- Widened 2026-08-27 to empty `scripts/` entirely, not only retire the CI guards.
  Scope now covers deleting the residual libraries/tests, relocating data files into
  `cli/` (then three, now two — `linkage-type-pairs.tsv` moved to delete-outright per
  the 2026-08-28 note below), and porting nine authoring guards to Python. Two
  interpretations
  a reviewer should weigh: (1) relocating `templates-schema.tsv` (and the
  `extract-work-items-cue-phrases.txt` drift file) touches production corpus code —
  scoped deliberately as relocation, not redesign (move the file, repoint the path);
  if that is judged out of scope, the Relocate-into-`cli/` bucket can be dropped and
  those files left in place.
  (2) The nine uncovered tests were placed in Python on the rule "test lives where
  the behaviour lives" — all nine lint authored plugin content, a Python build-system
  concern; none test migrated Rust logic. If a future lens/eval crate moves that
  logic into Rust, the placement would revisit. Backed by a three-agent audit of the
  full 49-file `scripts/` surface (reference graph, Rust coverage, test placement).

- Reviewed 2026-08-28 (review 1, verdict REVISE → revised). Changes: title broadened
  to name the `scripts/` emptying; the deferred bashisms decision made explicit in the
  Summary, with the thin-shell survivor-set and bash-3.2-verifier criteria marked
  contingent on the bashisms Open Question; config/hooks/decisions/github floor
  ownership pinned to this story
  outright (Requirement 1 and Technical Notes reconciled); an AC added for the Jira
  `external_id` cutover behaviour and its predecessor (0211/0212) named in
  Dependencies; the green→red invariants reframed as per-commit obligations. The
  co-delivery of guard retirement, data-file relocation and the nine-guard Python
  port is a **deliberate** choice, not a default — the three share the `scripts/`
  surface and the same lockstep/verification discipline; the disposition buckets keep
  a clean seam should a later split prove warranted.
- `doc_type_single_source.rs` disposition resolved 2026-08-28 by inspecting the test:
  its three cases split — one (`every_non_virtual_type_is_registered_exactly_once`)
  drives the compiled `config paths` resolver and is bash-free, the other two are
  pure bash-data oracles. Rescoped to keep the first (de-gated) and drop the other
  two. Knock-on: `linkage-type-pairs.tsv` had no production consumer
  (`corpus::linkage::TYPE_PAIRS` is a hand-maintained const that only mirrors it), so
  it moved from Relocate to Delete-outright, shrinking the relocation bucket to two
  files.
- Bashisms Open Question resolved 2026-08-28: the denylist is **not** dropped and its
  shell implementation is **not** kept — `scripts/lint-bashisms.sh` is reimplemented
  as a Python `tasks/` task, so shell no longer lints shell, and shfmt + ShellCheck
  are retained. All three are rescoped from the removed `shell_sources()` whole-tree
  walk to an explicit list of the ~3 surviving thin-shell files (launcher bootstrap,
  hook wrapper, Playwright executor). Rationale: the bash-3.2 floor is a proven,
  recurring macOS-only failure mode on invocation-critical-path shell, so an automated
  guard is worth keeping; porting the linter to Python removes the last shell-guards-
  shell irony while the heavy machinery (exec-bits, `SHELL_LIBRARIES`, suite floors,
  `check-scripts` release gate) still goes. The surviving checks run in the normal
  `mise run check` lint lane, not as a `check-scripts` release gate.

> Extracted from source documents without interactive enrichment, then reviewed
> across three passes (review 1) and promoted to `ready` on 2026-08-28.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0048, ADR-0049
- Prior research: `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
- Predecessors (all done): 0167, 0168, 0169, 0170, 0171, 0172, 0195, 0196, 0197,
  0211, 0212 — the domain-cluster migrations this cleanup follows.
- Rust successors of the retiring libraries: `cli/store/src/lib.rs`,
  `cli/corpus-adapters/src/{lock,jsonl}.rs`, `cli/vcs/src/{mode,classify}.rs`,
  `cli/corpus/src/doc_type.rs`, `cli/migrate-adapters/src/merge_move.rs`,
  `cli/corpus/src/frontmatter_validation/schema.rs`,
  `cli/work-cli/src/sync_author.rs`.
