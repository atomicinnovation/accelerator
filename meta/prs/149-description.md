---
type: "pr-description"
id: "149"
title: "[0299] Hold the CLI workspace's crates to ADR-0069's injection rules"
date: "2026-10-07T23:20:45+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0299"
parent: "work-item:0299"
relates_to: ["work-item:0226"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/149"
pr_number: 149
tags: ["cli", "architecture", "dependencies", "refactor"]
revision: "15d36a76e44005e564fc36daef38879c31ba9786"
repository: "accelerator"
last_updated: "2026-10-08T09:39:10+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0299] Hold the CLI workspace's crates to ADR-0069's injection rules

## Summary

Every crate in `cli/` now declares its ADR-0069 role, context and kind in
`[package.metadata.accelerator]`. A new `cargo metadata` lint, reached from
`cli:check` and `lint:check`, rejects every normal or build dependency the
ADR forbids, and the shipped workspace passes it with no exception list. To
get there, each of the 13 violating edges in 0299's table is replaced by a
port injected at a composition root, a move into the upstream domain, or a
move into a technical library. `consent-adapters` is gone, and the launcher
answers its SessionStart tracking question in-process, so `vcs tracking` and
the "was not checked" note are removed. The characterisation suite that
guarded the refactor is retired at the end, and the behaviours only it pinned
are now crate tests.

## Changes

### Enforcement

- **`tasks/lint/crate_dependencies.py`** — `lint:crate-dependencies:check`
  over `cargo metadata`. It enforces rules 1, 2, 4, 5 and 6. It also enforces
  three constraints the ADR leaves open: `kernel` depends on no workspace
  crate, a technical library depends only on `kernel` and other technical
  libraries, and only the launcher may depend on `accelerator-verify`. It runs
  a declaration check and a test-support check on top. Dev-dependencies are
  ignored, and rule 3 stays with review.
- **Declarations** — every `Cargo.toml` gains its role (`launcher` is a role
  of its own), domain and adapter crates their context, and each shared
  context's domain crate its closed set of downstreams.
  `tests/unit/tasks/test_crate_dependencies.py` pins the expected table and
  covers every fixture in the acceptance criteria.
- **Registration checklists** — both checklists in `tasks/README.md` gain the
  declaration step (the sub-binary checklist is now fourteen points).

### Ports and moves that remove the violations

- **`vcs` repository ports** — the `vcs` domain publishes file tracking and
  enclosing-repository roots as a port. `vcs-adapters` implements it, and
  `corpus-adapters`, `work-adapters` and `migrate-adapters` no longer depend
  on `vcs-adapters`. Their roots inject the port instead.
- **`consent-adapters` merged into `config-adapters`** — `config-adapters`
  depends on `vcs`, not `vcs-adapters`. The 7 roots that read consent keys
  inject tracking. This removes the packaging-only crate that rule 7 rules
  out.
- **Launcher tracking in-process** — the captured `vcs tracking --path`
  child, `launch/outbound/capture.rs`, `kernel::tracking`, `TrackingCheck`
  with `Unchecked`, and `AuditFinding::PersonalFileUnchecked` are deleted.
  A tracked `config.local.md` is now warned about in every session, offline
  included.
- **Tracker blocks in the `config` catalogue** — `config::tracker_block`
  describes `<tracker>.pull`/`.push`, and `tracker-support` derives its
  parsers from that description. The launcher drops `tracker-support`.
  `FilterSchema` was deleted once nothing read it.
- **`corpus` frontmatter port and validation pipeline** — the validation
  pipeline and `canonicalise_id`'s port move into `corpus`.
  `corpus-adapters` implements the parser and canonicaliser over `document`
  and `regex`. `research-adapters` drops `corpus-adapters`.
- **`migrate` through ports** — m0001 and m0008 parse through the `corpus`
  port and render canonically through a `MigrationContext` port that
  `migrate-adapters` implements over `document`, so `migrate` → `document`
  goes. Sync-baseline realignment moves to `work-adapters::sync::realign`,
  and `migrate-cli` composes it.
- **Technical-library moves** — the mkdir lock moves from `corpus-adapters`
  to `store::lock` with its own error type, so `jira-client` and
  `linear-client` drop `corpus-adapters`. `TEMP_PREFIX` moves to `kernel`, so
  the launcher drops `store`.
- **Rule 3 edges** — `work` → `config` (dead) and `collaboration` → `vcs`
  (now `collaboration::RepositoryOrigin`) are removed. `work` → `corpus` and
  `work` → `tracker` are kept: `work`'s model is expressed in their types.

### Safety net and measurement

- **Characterisation suite, then its retirement** — a pytest suite of 541
  cases over the built binaries, with 534 goldens, guarded every refactor
  phase. Its goldens stayed byte-identical throughout, apart from the four
  "without `accelerator-vcs`" summary goldens, which moved from the
  `Unchecked` note to the in-process answer, and two new unknown-subcommand
  goldens for `vcs tracking`. With the refactor done, an overlap audit found
  492 of the goldens already covered by crate tests, and git/jj pairs
  byte-identical everywhere but the metadata revision. The roughly 25
  behaviours only the suite pinned are now 45 Rust tests across 14 crates.
  The suite, its `test:integration:characterisation` lane and its
  `build:cli:characterisation:dev` build are removed.
- **Per-sub-binary dev builds** — `build:cli:<x>:dev` tasks
  (`tasks/shared/dev_builds.py`) let each integration lane build only the
  binaries it runs.
- **Launcher measurement** — the `measure` tasks gain launcher size and
  SessionStart summary latency. `deny.toml`'s symbol-count table and `uluru`
  MPL-2.0 record are rewritten to match the re-measured binaries.

### Fixes found along the way

- **Runner working directory** — `BashCommandRunner` rejects a base reached
  through a repository symlink by folding `outside_the_repository` into
  `inside_the_repository`.
- **Visualiser watch registration** — `watcher::spawn` registers on a
  blocking thread behind a `DirectoryWatcher` port, so the server accepts
  connections before slow `FSEvents` watches finish. `sse_e2e.rs` now waits
  for the event within a 300 s budget.
- **e2e health port** — `E2E_HEALTH_PORT` is allocated per run, so two
  workspaces can run `mise run` concurrently.
- **docs-site advisories** — `sharp` 0.35.5 (GHSA-wq5f-xc86-pv6w) and
  `source-map-js` 1.2.2 (GHSA-68fv-2mgg-jv7q).

## Context

- Work item: `meta/work/0299-bring-the-cli-workspace-s-crate-dependencies-into-line-with.md`
- ADR: `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
  (accepted in this PR)
- Plan: `meta/plans/2026-10-06-0299-crate-dependencies-adr-0069.md`. Its
  Implementation Notes record every phase's deviations, the rule 3 outcomes,
  and the before/after figures.
- Research: `meta/research/codebase/2026-10-06-0299-crate-dependencies-adr-0069.md`

## Testing

- [x] `mise run lint:crate-dependencies:check` passes over the shipped
  workspace. Its 54 unit tests pass.
- [x] Injecting `jira-client` → `corpus-adapters` failed `mise run check` in
  that lint alone, naming `jira-client -> corpus-adapters: rule 4` (recorded
  in Phase 12).
- [x] The characterisation suite passed with no golden changed after each
  refactor phase, apart from Phase 7's reviewed goldens.
- [x] Each ported crate test failed when the behaviour it guards was
  broken, either in production code or, for a few, by changing the expected
  value. Every break was reverted, and the stack changes no production code.
- [x] `mise run` exited 0 over the final stack, with the suite removed
  (585 s, macOS). It also exited 0 at the end of Phase 13.
- [x] Manual migrate check: all 10 migrations applied to a scratch corpus, a
  second run reported none pending, and the result passed validation.
- [x] Launcher size gate: 16 667 568 bytes, ratio 2.07 against 8 065 488
  (gate 10).
- [x] Summary latency gate: median ratios 0.18–0.25 across git, jj,
  colocated jj and large jj, cold and warm. The summary no longer spawns
  `accelerator-vcs`.
- [x] Visualiser symbol gate: `accelerator-visualiser` links none of `gix`,
  `jj-lib` or `uluru`.
- [ ] Warm-dispatch after figure: this needs the `main` pipeline's
  prerelease, so it is measured after merge against the 44.00 ms before
  figure.

## Notes for Reviewers

- ⚠️ The launcher now links the `gix`/`jj-lib`/`uluru` closure (1662 / 2896 /
  3 symbols). That is the cost of in-process tracking and why the binary
  doubled. `deny.toml` now names 10 binaries that carry the MPL-2.0 notice
  obligation, the launcher among them. `notices:update` left the artefact
  unchanged.
- `accelerator vcs tracking` is removed and now fails as an unknown
  subcommand. Only the launcher called it. `CHANGELOG.md` is not touched in
  this PR.
- The suite's goldens are added and then removed within this PR, so the net
  diff carries none of them. The review surface is the `cli/` crates,
  `tasks/lint/crate_dependencies.py` and `tasks/shared/`. The
  frontmatter fixture corpus moved to `cli/corpus-cli/tests/fixtures/`.
- The overlap audit surfaced two gaps that are left for follow-up, not
  pinned. `accelerator-jira` and `accelerator-linear` check only a `pull`
  block's shape and ceilings, so they accept blocks that `config dump` and
  `work sync` refuse. Nothing guards that the SessionStart summary never
  spawns `accelerator-vcs`.
- The read-only-directory tests and the canonicalised `/private/var` paths
  in the ported tests have only run on macOS; CI is the first Linux run.
- `meta/reviews/prs/149-review-1.md` records the multi-lens review of this
  PR before the suite was retired.
- The only remaining normal adapter → adapter edges across contexts are
  `jira-client` and `linear-client` → `tracker-support`. The declared
  downstreams of the shared `tracker` context allow them.
- `accelerator-corpus`'s `gix_` count rose from 546 to 2177 once it composed
  `InProcessProbe` behind the `vcs` ports. It already linked all three
  crates.
