---
type: "plan-validation"
id: "2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename-validation"
title: "Validation Report: Bootstrap Self-Location and the ACCELERATOR_PLUGIN_ROOT Rename"
date: "2026-07-29T13:36:14+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
parent: "work-item:0182"
target: "plan:2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename"
tags: ["validation", "cli", "launcher", "bootstrap", "plugin-root", "hooks", "lint-guards"]
last_updated: "2026-07-29T15:21:21+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Bootstrap Self-Location and the `ACCELERATOR_PLUGIN_ROOT` Rename

Validated at revision `2da813f0` (working copy empty, parent `b89ff3a5`).
Every automated criterion was re-run rather than taken from the plan's own
annotations; every count below is measured in this session.

**Amended 2026-07-29T15:21Z**, after `1.24.0-pre.17` was installed: Phase 1a's
release-candidate manual criteria are now performed and passing against the
published artifact. Phase 3's five remain deferred — see *Manual Testing*.

### Implementation Status

✓ Phase 0: Determinations — fully implemented
✓ Phase 1a: Bootstrap self-location and `--fail-safe` — fully implemented
✓ Phase 1c: Work-item seam and build edge — fully implemented
✓ Phase 1b: The rename — fully implemented
✓ Phase 2: The `CLAUDE_*` boundary guard — fully implemented
✓ Phase 3: Terminal invocation surface — fully implemented
✓ Phase 4: `!`-site conformance suite — fully implemented
✓ Phase 5: A missing plugin root becomes a named error — fully implemented
✓ Closing step (`mise.local.toml` removal) — **done 2026-07-29**, every criterion
performed. The file was already absent when the deletion was attempted (removed
alongside the `1.24.0-pre.17` install, outside this session), and `mise env` now
exports no plugin root. Details under *Closing Step Results*.

**The installed artifact is the Phase 1a-only release**, which is the plan's
independent-releasability claim confirmed in the field rather than argued:
`1.24.0-pre.17`'s `bin/accelerator` carries the self-location block
(`BASH_SOURCE`, `max_hops=16`, `export ACCELERATOR_PLUGIN_ROOT`) **and** the
transitional `export CLAUDE_PLUGIN_ROOT` at `:116` that Phase 1b deletes — so it
predates 1b, and it works because the shipped launcher still reads the old name.
Phase 3's hook is absent from it, which is why those checks cannot yet run.

Sixteen commits carry the work (`nstyplkt`…`pnynskrr` plus the pre-existing-flake
fix), one more than the plan's progress table records.

### Closing Step Results

Performed 2026-07-29, after `1.24.0-pre.17` was installed.

✓ `test ! -e mise.local.toml` — the file was **already gone** when deletion was
  attempted (`rm` reported "No such file or directory"), removed outside this
  session alongside the plugin update. `mise env` exports no plugin root.
✓ `.gitignore:26` carries the entry, and the file is absent from `main`.
✓ Shell suites with **no ambient root**: `test:integration:work` 404 assertions
  pass, `test:integration:config` 1692, each exit 0. Run as two separate
  invocations — a single `mise run a b` passes `b` as an *argument* to `a`, which
  fails with invoke's "No idea what … is!" rather than running the second task.
✓ `mise run` (bare default) exits 0 end-to-end under
  `env -u CLAUDE_PLUGIN_ROOT -u ACCELERATOR_PLUGIN_ROOT`, with all three
  `cli/`-scoped guards executing and the formatters changing nothing.
⚠️ An earlier full run of the same commit failed `test:integration:config` on a
  teardown race in the Playwright executor cases — `rm: … Directory not empty` on
  the fixture's `.accelerator/tmp`. Not root-related and not caused by the
  deletion: that suite passed standalone both before and after, and green inside
  the clean run. It belongs to the recorded config-suite flake class. Worth its
  own item if it recurs, since it turns a full local run red at random.
✓ Two CLI-invoking skills load against installed `1.24.0-pre.17` with no error
  and no permission prompt: `list-work-items` (injections populated — `meta/work`,
  `linear`, the work-item template) and `paths` (all 14 configured paths
  resolved). Non-empty injections are what distinguishes a real read from a
  `--fail-safe` degradation.

  **Caveat on that last one.** This session's own environment still carries a
  stale `CLAUDE_PLUGIN_ROOT` pointing at `1.24.0-pre.16`, inherited at session
  start, so the `!` shells did see one. It is provably inert — the executed path
  is `pre.17`'s own bootstrap, whose only mention of the variable is the
  transitional `export` at `:116`, with no read anywhere — but the unconfounded
  version of this check is a session started outside this repository's mise
  context. Worth doing once; it is the only check here resting on an argument
  rather than on an isolated environment.

### Automated Verification Results

✓ `mise run` (bare default) exits 0 end-to-end — the full local CI mirror, run
  to completion in this session. Working copy is still empty afterwards, so the
  in-place formatters had nothing to change.
✓ `mise run cli:check` exits 0. At the time of the first pass this had to be run
  separately, because the bare default did **not** reach it; the wiring described
  under *Potential Issues* has since fixed that, and the clean closing-step run
  shows all three guards executing inside `mise run`.
  `lint:claude-coupling:check` completes in 228ms.
✓ The `CLAUDE_*` guard is clean and its scope is intact: `_in_scope()` discovers
  **724** files and `violations(REPO_ROOT)` returns `[]` — reproduced directly,
  matching the plan's measurement exactly.
✓ Suite counts, all from the full run: `test:integration:entrypoint` 46 passed ·
  `test:integration:hooks` 22 passed · `test:integration:skill-invocation` 128
  passed · `test:unit:tasks` 406 passed, 409 after the guard-wiring cases ·
  `test:integration:tasks` 51 passed ·
  `test:integration:deny` 15 passed · `test:integration:dev` 17 passed ·
  `test:unit:cli` green (89.12% line coverage) · all shell suites (`config`,
  `work`, `migrate`, `integrations`, `decisions`, `github`) green · frontend
  unit + E2E green.
✓ Phase 5's named cases all executed in the run:
  `templates_list_with_no_plugin_root_is_a_named_refusal_not_an_empty_table`,
  `a_missing_plugin_root_refuses_identically_with_and_without_fail_safe`,
  `a_user_override_still_resolves_with_no_plugin_root`,
  `an_empty_plugin_root_refuses_to_compose`,
  `plugin_root_unavailable_names_the_variable_and_the_bootstrap`,
  `the_root_independent_families_still_succeed_with_no_plugin_root`.
✓ `! grep -q 'CLAUDE_' bin/accelerator` — zero matches; the transitional export
  is gone.
✓ `! grep -q 'CLAUDE_PLUGIN_ROOT' hooks/launcher-link-refresh.sh` — zero
  matches, header comment included.
✓ Residue purge holds: the non-`meta/` tracked `CLAUDE_PLUGIN_ROOT` grep returns
  **87** files, every one inside the permitted set the plan enumerates (no
  file outside it). Re-derived by running the grep, not by transcription.
✓ Phase 3 documentation criteria: `docs/internals.md:88` carries
  `## Terminal Invocation`; `README.md:57-58` links it and the gloss names
  "running the CLI from a terminal"; no unexpanded `CLAUDE_PLUGIN_DATA}/bin/…`
  token in the docs; `accelerator-visualiser` and the competing `~/.local/bin`
  recipe are both gone.
✓ `hooks/hooks.json` registers `launcher-link-refresh.sh` as the **fourth**
  `SessionStart` group (index 3), so `test-vcs-detect.sh`'s hard-coded
  `SessionStart[0]` assertion is undisturbed; the hook is mode 0755.
✓ Behaviour spot-check against a freshly built launcher from a bare temp dir
  with both root variables stripped: `config templates list` exits 1 with
  `the plugin installation root is unknown: set ACCELERATOR_PLUGIN_ROOT, or
  invoke accelerator through bin/accelerator, which derives it`; the same command
  with `--fail-safe` still exits non-zero with **0 bytes** on stdout;
  `config summary` still exits 0.

Nothing failed. No automated criterion in the plan was found to be overstated.

### Code Review Findings

#### Matches Plan:

- `bin/accelerator:21-115` implements the argv scan (stops at the first `--` and
  first match), `max_hops=16`, `fail`/`fail_integrity` as two named entry points,
  `dir_of` by parameter expansion, the `while [[ -L ]]` chase with `|| fail` on
  every `readlink`/`cd`, `cd -P` for the `..` step, `CDPATH=`, the single-line
  sanitiser `${1//$'\n'/ }`, and one write-only `export ACCELERATOR_PLUGIN_ROOT`.
  The file reads no root variable at all.
- `hooks/launcher-link-refresh.sh` matches the pinned listing line for line:
  single `finish()` exit point, unverified-log read *above* the
  `CLAUDE_PLUGIN_DATA` guard, `[ -L ]`/`[ ! -d ]` before `mkdir -p -m 0700`, the
  symlink-to-directory clear before the staged `mv`, the `STAGED` pre-check with
  an `rm -rf` trap, the post-`mv` `[ -L ]` assertion, and the `PREVIOUS`
  comparison that reports a re-point. Both deliberate shellcheck disables carry
  their justification.
- `tasks/lint/claude_coupling.py` is shaped as specified: pure `violations(root)`,
  no allowlist, decode-failure skip with `_SKIP_SUFFIXES` as a speed hint only,
  fail-closed on empty *discovery* (`_MIN_SCANNED = 200`) **and** on any `_FILES`
  entry that no longer resolves, `path:line:text` reporting, and a docstring
  stating the invariant plus the out-of-scope-by-construction adapter layer.
- `tasks/shared/sources.py:63-86` promotes `walk_files(repo, subtree, prune)`
  with replace semantics and the root-spec-not-subtree-spec rule documented;
  `shell_sources()` and `test_python_coverage.py`'s `_py_files` both ride it.
- Phase 5's consumer placement is correct where it matters:
  `FileConfigStore::resolve` checks the user override *before*
  `require_plugin_root()`, so `a_user_override_still_resolves_with_no_plugin_root`
  is testing a real seam rather than an accident; `is_refusal()` is an exhaustive
  `match` (a new variant will not compile until classified) and is `pub` so the
  launcher's `From<ConfigError> for Failure` can reach it.
- `tests/integration/support/installation.py:301-319` enforces the hermeticity
  preconditions inside the single funnel, with the URL check implemented as
  `urlparse(...).hostname.endswith(".invalid")` rather than a suffix test on the
  whole URL — the stronger form the plan argues for.
- `tests/unit/tasks/test_mise.py` pins both `_LAUNCHER_DEPENDENTS` and an
  exhaustive `_NO_LAUNCHER_NEEDED`, with `test:integration:skill-invocation`
  in the latter and a reason recorded.
- `.gitignore` covers the launcher cache, sub-binary cache, lock, unverified log
  and staged shims, with the digest-anchored shim pattern and a comment
  explaining why `-*-*` would have been wrong; the stale pre-0168
  `visualise/bin/…` entry is gone.

#### Deviations from Plan:

- **The plan document was internally stale about its own last phase** — the
  mergeability table said Phase 5 "not started" and the *Progress — 2026-07-29*
  paragraph said it remained, listing fifteen commits, while Phase 5's own
  section recorded it done and the commit (`Refuse rather than answer empty when
  the plugin root is unknown`) existed. **Corrected in this validation pass**: the
  table row reads done, the paragraph records all eight phases complete with only
  the closing step and the release-candidate manual checks outstanding, and the
  sixteenth commit is listed.
- **The empty-string root filter landed in `with_plugin_root`**
  (`cli/config-adapters/src/store.rs:65-69`), not at `cli/launcher/src/main.rs:176`
  as Phase 1b §1 specified. Strictly better — one filter now serves the launcher
  *and* the visualiser server — and Phase 5's implementation notes recorded it,
  but Phase 1b's text did not. **Corrected in this validation pass**: Phase 1b §1
  now names `with_plugin_root` as where the rule lives and says `main.rs`'s
  `var_os` is deliberately left unfiltered.
- **`violations()` gained a `min_scanned` parameter** not in the plan's signature,
  as a seam for the guard's own tiny-tree tests. The real-tree assertion uses the
  default, so the floor is not weakened.
- **`.gitignore` uses `bin/.tmp-*`** where the plan specifies
  `bin/.tmp-launcher-*` — broader than asked, harmless in a directory the plugin
  ships from.
- **Two closing-step criteria are already met but unticked**: `.gitignore:26`
  carries `mise.local.toml`, and the file is absent from `main`. Beyond that,
  `mise.local.toml` is no longer tracked in the working-copy commit at all
  (`jj file list mise.local.toml` returns nothing), so the *Current State
  Analysis* claim that it "exists only in the current jj working-copy commit" is
  already out of date — only the on-disk, untracked file remains, and its sole
  remaining effect is the one the plan describes: supplying `CLAUDE_PLUGIN_ROOT`
  to the still-unfixed installed bootstrap.
- Phase 3's suite shipped as Python (`tests/integration/hooks/test_launcher_link_refresh.py`,
  22 cases) rather than a `hooks/test-*.sh` harness, and Phase 4 did the support
  extraction Phase 1a was supposed to leave behind. Both are recorded in the
  phases' own *Implementation notes*; noted here only for completeness.

#### Potential Issues:

- **The three `cli/`-scoped guards were unreachable from `mise run`.** Measured
  from the full run's task list: `lint:claude-coupling:check`,
  `lint:store-duplication:check` and `lint:vendor-shims:check` did not execute.
  They rode in `cli:check` only, which CI runs as its own step
  (`.github/workflows/main.yml:257`) — so the invariant *was* enforced before
  merge, but `CLAUDE.md`'s "done means `mise run` exits 0" was not a complete
  local gate for it. The plan deferred this ("worth doing, but as its own
  change") and no follow-up item covered it (0183 is the
  `migrate-discoverability` stderr advisory, 0184 is `template_names` on a
  known-but-wrong root).

  **Closed in this validation pass, on the maintainer's decision to wire rather
  than ticket**: all three are now in `lint:check.depends` (`mise.toml:462`) as
  well as `cli:check.depends`, so the bare `default` task reaches them —
  `default` depends on `lint:check`, never on `check`, which is the specific
  reason a `cli:check`-only guard was invisible locally. A second paired
  assertion, `test_gate_wired_into_lint_check`, pins the new placement over the
  same constant as the `cli:check` one; falsified by removing the three entries
  (3 failed, 23 passed) and green restored. `mise run lint:check` exits 0 with
  all three executing, and `tasks/README.md` now records both placements and
  why. `test_mise.py` 26 passed; `mise run build-system:check` exits 0.
- **`dir_of()` assigns `dir` as a global** (`bin/accelerator:74`). Harmless today
  — the value is recomputed on every call and nothing else in the file uses the
  name — but the file's other helpers keep no state, so a later reader could
  reasonably assume this one doesn't either. `local dir` is bash-3.2-safe.
- `fail_integrity()` (`:54`) calls `dir_of()` (`:71`) defined below it. Correct at
  runtime, and unreachable before both definitions exist, but the ordering is the
  one thing in the header region that would break silently if a future gate moved
  above line 71.
- Phase 4's suite excludes the visualiser daemon `!` site by design, so
  external-subcommand dispatch has no `!`-site-level coverage; the suite's
  docstring records this and points at `version.rs`/`cache_root.rs`. Accepted, but
  it means the one `!` site that *hard-requires* the plugin root is verified only
  at the Rust layer.

### Manual Testing Required:

Phase 1a's deferrals are **discharged**; Phase 3's still stand, because the hook
is not in the installed artifact.

1. Phase 1a, performed 2026-07-29 against installed `1.24.0-pre.17`, invoked by
   absolute path from a bare `mktemp -d` with **both** root variables stripped:
  - [x] `env -u CLAUDE_PLUGIN_ROOT -u ACCELERATOR_PLUGIN_ROOT <root>/bin/accelerator config templates list`
        exits 0 and renders
        `` | `adr` | plugin default | `<plugin>/templates/adr.md` | `` — a real
        plugin-default row, not the empty table this bug produced — with **zero**
        `accelerator:` lines on stderr.
  - [x] The originally reported failing command,
        `config instructions commit --fail-safe`, exits 0 with no `accelerator:`
        diagnostic. Empty stdout is correct here, as the plan records.
  - [x] The genuine fetch → verify → cache chain ran against the real release:
        `bin/accelerator-launcher-1.24.0-pre.17-darwin-arm64` and its `.minisig`
        are cached in the installed tree, and no
        `.accelerator-unverified.log` exists — so nothing degraded to an
        unverified path.

2. Phase 3, still deferred — `hooks/launcher-link-refresh.sh` is **absent** from
   `1.24.0-pre.17`, so a release carrying Phase 3 is needed. Current machine
   state, recorded as the pre-condition baseline: `${CLAUDE_PLUGIN_DATA}` resolves
   to `~/.claude/plugins/data/accelerator-atomic-innovation-prerelease/` (the
   `data/<plugin>-<marketplace>` shape the documentation describes as *typical*),
   it holds no `bin/`, and there is no `~/.local/bin/accelerator`.
  - [ ] In a real session the hook fires at `SessionStart` and
        `${CLAUDE_PLUGIN_DATA}/bin/accelerator` resolves to the current
        installation.
  - [ ] The documented recipe, followed verbatim on a machine without
        `~/.local/bin`, yields a working `accelerator` on `PATH` in a new login
        shell.
  - [ ] After a plugin upgrade, a new session's link points at the new root with
        no user action.
  - [ ] Two concurrent sessions never leave the link absent.
  - [ ] Removing the plugin leaves only the dangling plugin-owned link inside
        `${CLAUDE_PLUGIN_DATA}`.

3. Closing step — **done**, see *Closing Step Results* above. One optional
   follow-up remains:
  - [ ] Re-confirm a CLI-invoking skill load from a Claude Code session whose
        environment carries no `CLAUDE_PLUGIN_ROOT` at all (start it outside this
        repository's mise context), so the check rests on isolation rather than
        on the inertness argument.

### Recommendations:

- ~~Correct the plan's mergeability table and progress paragraph~~ — **done in
  this pass**, along with the two closing-step criteria that were already
  satisfied (now ticked, with `.gitignore:26` named) and the *Current State
  Analysis* and *Closing step* lines about `mise.local.toml` sitting in the
  working-copy commit (it is now untracked; only the on-disk file remains).
- ~~Correct Phase 1b §1's text about the empty-string filter~~ — **done in this
  pass.**
- ~~Close the default-task blind spot~~ — **done in this pass** by wiring rather
  than ticketing; see *Potential Issues*. Note this deliberately overrides the
  plan's Phase 2 argument against dual-wiring (which reasoned from `cli:check`
  being what CI runs, and did not weigh the bare `default` task's blindness).
- ~~Do the closing step~~ — **done in this pass**, all five automated and both
  manual criteria performed; see *Closing Step Results*.
- **Do not flip the plan to `done` yet.** Phase 3's five checks still need a
  release carrying `hooks/launcher-link-refresh.sh`, which `1.24.0-pre.17` does
  not have; `in-progress` stays accurate until then. That, plus the optional
  clean-session skill load, is all that is left.
- **Consider an item for the config-suite teardown race** if it recurs — it turned
  one full local run red at random during this pass, and a flake in the gate that
  defines "done" costs more than its size suggests.
- Optionally, make `dir` local in `dir_of()` while `bin/accelerator` is still
  fresh in mind.
