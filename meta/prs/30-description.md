---
type: "pr-description"
id: "30"
title: "[0182] Plugin root rename and terminal invocation surface"
date: "2026-07-29T16:18:51+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0182"
parent: "work-item:0182"
relates_to: ["plan:2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename", "plan-validation:2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename-validation", "work-item:0184"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/30"
pr_number: 30
tags: ["pr", "cli", "launcher", "plugin-root", "hooks", "lint-guards"]
revision: "517db8c33c1a7964d22dbdf982aa32270a803545"
repository: "accelerator"
last_updated: "2026-07-29T16:18:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0182] Plugin root rename and terminal invocation surface

## Summary

PR #28 made `bin/accelerator` derive its installation root from its own location, unbreaking all 45 CLI-invoking skills. This is the rest of that work: the launcher and server layers move off `CLAUDE_PLUGIN_ROOT` onto `ACCELERATOR_PLUGIN_ROOT`, a lint guard makes that boundary non-negotiable, an unknown plugin root becomes a named refusal instead of a silently empty answer, and running `accelerator` from an ordinary terminal becomes a documented, upgrade-surviving surface.

## Changes

**The rename.** `cli/launcher`, `cli/visualiser/server` and the cache-root resolver read `ACCELERATOR_PLUGIN_ROOT`; the four out-of-tree writers (`tasks/dev.py`, `tasks/shared/dev/circus.py`, `tasks/test/helpers.py`, `tests/integration/dev/dev_integration_driver.py`) set it. `bin/accelerator` drops the transitional dual export it carried for one release, so nothing in the rename set names a `CLAUDE_*` variable any more. An empty value is now treated as unset, filtered once in `FileConfigStore::with_plugin_root` so the launcher and the server inherit the rule from one place.

**A missing root is a named error.** `ConfigError::PluginRootUnavailable` names the variable and the remedy, and `is_refusal()` classifies every variant exhaustively — a new variant will not compile until it is classified. The template ports became fallible, so a rootless `config templates list` exits non-zero with a diagnostic instead of printing an empty table at exit 0, and `templates eject --all` refuses instead of silently ejecting nothing. Refusals are byte-identical with and without `--fail-safe`. Root-independent families — `paths`, `summary`, `instructions`, `context`, `work`, `review` — keep working without a root, and a user template override still resolves without one.

**A boundary guard.** `tasks/lint/claude_coupling.py` fails if anything in the rename set (`cli/**`, `bin/accelerator`, the four writers) names a `CLAUDE_*` variable. No allowlist: the failure message says the reference must be removed. It fails closed on empty discovery, on a file count below a floor, and on any named path that no longer resolves. The gitignore-honouring directory walk it needs was promoted to `tasks/shared/sources.py:walk_files` rather than copied a third time. The guard is wired into both `cli:check` (what CI runs) and `lint:check` (the only path the bare `mise run` reaches), together with the two pre-existing `cli/`-scoped guards, which had the same blind spot.

**Terminal invocation.** A new `SessionStart` hook, `hooks/launcher-link-refresh.sh`, keeps `${CLAUDE_PLUGIN_DATA}/bin/accelerator` pointing at the current installation's launcher, so a symlink you create once on your `$PATH` survives upgrades. It exits 0 on every path, refuses rather than clobbering anything it does not own, reports a re-point, and surfaces an unverified-launcher record from the bootstrap's durable log. `docs/internals.md` gains a Terminal Invocation section (discovery-first, since the data-directory layout is an undocumented internal); `docs/releases-and-compatibility.md` covers channel switches; the competing version-pinned recipe in the visualise skill and a stale `accelerator-visualiser` reference are gone.

**A conformance suite for the `!` sites.** `tests/integration/skill-invocation` runs all 122 distinct `config` commands the skills invoke at load time through the real bootstrap against a fixture installation, in the production shape — correct absolute path, empty environment — which is the one configuration no other suite exercised, and the gap that let this bug ship. Plus six structural invariants over the corpus. The fixture-installation apparatus was extracted to `tests/integration/support/` and the entrypoint suite refactored onto it.

**Test-infrastructure repairs.** The work-item template seam now forces failure through `ACCELERATOR_BIN` rather than a bogus plugin root, with a paired test proving the fallback branch is actually entered — it would have passed vacuously after the rename. `hooks`, `decisions` and `github` gained shell-suite discovery floors via one extracted helper. `test_mise.py` pins which integration tasks need a prebuilt launcher, exhaustively, so a new task forces a decision.

## Context

- Work item: [`meta/work/0182-cli-derives-plugin-root-from-own-location.md`](../work/0182-cli-derives-plugin-root-from-own-location.md)
- Plan: [`meta/plans/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename.md`](../plans/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename.md)
- Validation: [`meta/validations/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename-validation.md`](../validations/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename-validation.md)
- Predecessor: PR #28 (bootstrap self-location and `--fail-safe`), released as `1.24.0-pre.17`
- Follow-up raised here: [`meta/work/0184-template-enumeration-swallows-a-wrong-plugin-root.md`](../work/0184-template-enumeration-swallows-a-wrong-plugin-root.md)

## Testing

- [x] `mise run` (bare default) exits 0 end-to-end, run under `env -u CLAUDE_PLUGIN_ROOT -u ACCELERATOR_PLUGIN_ROOT` so no ambient root reached any task. The formatters changed nothing. The commits after that run are documentation-only, so the tip's code is exactly what was verified.
- [x] `mise run cli:check` exits 0; `lint:claude-coupling:check` scans 724 files with 0 violations in ~230ms, and its scan descends into neither `cli/target/` nor `node_modules/` nor `dist/`.
- [x] Shell suites with no ambient plugin root: `test:integration:work` 404 assertions, `test:integration:config` 1692, each exit 0.
- [x] `test:integration:skill-invocation` 128 passed (122 command cases + 6 corpus invariants); `test:integration:entrypoint` 46; `test:integration:hooks` 22; `test:unit:tasks` 409; `test:unit:cli` green.
- [x] Guard falsifiability: reintroducing a `CLAUDE_PLUGIN_ROOT` read into `cli/launcher/src/main.rs` and the transitional export into `bin/accelerator` both turn `cli:check` red with a `path:line:text` report; removing the three `lint:check` entries reddens the new placement assertion.
- [x] Rootless behaviour, measured against a freshly built launcher from a bare temp directory with both variables stripped: `config templates list` exits 1 naming `ACCELERATOR_PLUGIN_ROOT`, 0 bytes on stdout with and without `--fail-safe`; `config summary` and `config path work` still exit 0.
- [x] Against the installed `1.24.0-pre.17` (the predecessor release), rootless invocation by absolute path renders the plugin-default `adr` row and the originally reported `config instructions commit --fail-safe` exits 0 clean.
- [ ] The terminal link end-to-end — hook firing at `SessionStart`, the documented recipe on a machine without `~/.local/bin`, surviving an upgrade, two concurrent sessions, and plugin removal. Needs a release carrying the hook; `1.24.0-pre.17` predates it. The hermetic equivalents are the 22 hook cases.
- [ ] One skill load from a Claude Code session whose environment carries no `CLAUDE_PLUGIN_ROOT` at all. Two skills load correctly today, but this session inherited a stale value at startup, so that check currently rests on the bootstrap never reading one rather than on isolation.

## Notes for Reviewers

**Where to look first.** `bin/accelerator`'s self-location block landed in #28; what is new here is the removal of its second `export`. The two files carrying the most judgement are `hooks/launcher-link-refresh.sh` (every guard exists for a measured filesystem behaviour — in particular `mv -f` onto a symlink-to-directory writes *inside* it, which is why the link is cleared before the rename) and `cli/config-adapters/src/store.rs`, where the tier check must stay below the user-override check or a rootless override stops resolving.

**Bootstrap and launcher must ship together, and do.** The bootstrap no longer exports the old name and the launcher no longer reads it, so a new bootstrap paired with a pre-rename launcher would find no root. That pairing cannot occur: the launcher cache is version-keyed and the bootstrap fetches the launcher matching `plugin.json`. Worth confirming during review, since it is the obvious failure mode of a two-sided rename.

**A changelog gap.** `Unreleased` gained the terminal-invocation entry but has nothing for the user-visible behaviour change in this PR — a rootless template command now fails loudly where it used to print an empty table at exit 0 — and no `Fixed` entry for the original bug. Both are arguably worth adding before release; I left the changelog as the branch had it rather than editing it inside a description pass.

**One decision overrides the plan.** The plan argued the `cli/`-scoped guards should ride in `cli:check` *only*, reasoning from CI. Validation measured the consequence: because the bare `default` task depends on `lint:check` and never on `check`, none of the three guards ran in a full local run, so a boundary violation was green locally and caught only in CI. All three are now dual-wired, which avoids the "third pattern" the plan was guarding against. The plan records the reversal at the point of the original argument.

**Known gaps, both recorded rather than fixed.** The conformance suite excludes the visualiser daemon `!` site — the only one that hard-requires the plugin root — because it verifies `manifest.json` against the real release key; it is covered at the Rust layer instead. And `test:integration:config` failed once during validation on a teardown race in the Playwright executor cases (`rm: … Directory not empty`), passing standalone before and after; it belongs to the recorded config-suite flake class, not to this change, but it can turn a full local run red at random.

**The hook is shell, not CLI logic**, which diverges from ADR-0048. The justification is circularity — the link exists to make the launcher reachable, so maintaining it cannot depend on the launcher being fetched, verified and cached — and it is recorded in the plan's Phase 3 overview rather than left as apparent drift.
