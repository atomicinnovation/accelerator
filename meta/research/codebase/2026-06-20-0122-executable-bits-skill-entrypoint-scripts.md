---
type: codebase-research
id: "2026-06-20-0122-executable-bits-skill-entrypoint-scripts"
title: "Research: Executable-bit audit and a library-list guard for skill entrypoint scripts (0122)"
date: "2026-06-20T16:43:31+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0122"
parent: "work-item:0122"
relates_to: ["work-item:0106", "work-item:0098", "work-item:0107"]
topic: "Audit and correct missing executable bits on skill entrypoint scripts; library-list guard"
tags: [research, codebase, scripts, permissions, executable-bit, lint, shell, ci]
revision: "aa8c4ad505f7a69d44aab9df13ce5b8295122d56"
repository: "build-system"
last_updated: "2026-06-20T16:43:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Executable-bit audit and a library-list guard for skill entrypoint scripts (0122)

**Date**: 2026-06-20T16:43:31+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: aa8c4ad505f7a69d44aab9df13ce5b8295122d56
**Branch**: detached / jj working copy (build-system workspace)
**Repository**: build-system

## Research Question

Work item 0122 proposes (a) a checked-in **library-list** manifest of sourced-only
`.sh` scripts, (b) correcting every tracked `.sh` so that *executable ⟺ off the
library-list*, and (c) a bidirectional CI guard in the `lint:shell` family that
enforces that invariant via `shell_sources()`. This research grounds the work item
against the live tree: it audits the real executable bits, classifies every
currently-`644` script as library vs entrypoint, verifies the seed set and the
four named suspects, maps the existing shell-lint task/test machinery the guard
must slot into, and surfaces the populations the work item under-scopes.

## Summary

The work item's *design* is sound and its mechanism (library-list + bidirectional
guard via `shell_sources()`) fits the existing infrastructure cleanly. But its
**scope estimate is materially too small**, and there is one **unanticipated
population** that will break the guard as specified. The key findings:

1. **The true correction set is ~23 mode changes, not the 6 the work item names.**
   - **18 entrypoints missing `+x`** (`644 → 755`), not 2. Beyond the named
     `migrations/0004-…sh` and `test-interactive-protocol.sh`, there are **6 jira
     `*-flow.sh`** scripts and **10 `work-item-*.sh`** scripts at `644` that are
     invoked by bare path from SKILL.md bodies and by `bash` from sibling scripts
     — invoked *identically* to their `755` jira/linear siblings. They are genuine
     inconsistencies the audit must catch.
   - **5 libraries carrying `+x`** (`755 → 644`), not 4. The work item names
     `atomic-common.sh`, `config-common.sh`, `vcs-common.sh`,
     `work-item-common.sh`; the audit found a **fifth**:
     `skills/visualisation/visualise/scripts/test-helpers.sh` (sourced-only, yet
     `755`, directly contradicting `tasks/test/helpers.py:20-22`).

2. **Migration test-fixtures are an unhandled population.** Eight `.sh` files under
   `skills/config/migrate/scripts/test-fixtures/**` are at `644`. They are
   executed only via `bash "$f"` (the migration runner globs by `find -name`, never
   by exec bit) so they never need `+x` — yet `shell_sources()` **keeps fixtures**,
   so the guard *will* see them. As specified, the bidirectional guard would demand
   `+x` on all eight. They need a fixture exemption or library-list entries; flipping
   them to `755` would be a wrong, cosmetic no-op for fixtures.

3. **"Sourced ⟹ library" is not safe on its own — three dual-use scripts prove it.**
   `linkage-parser.sh`, `validate-source.sh`, and `jira-fields.sh` are each
   `source`d by their own tests *and* invoked by path in production. The work item's
   AC1 procedure ("≥1 source ref **and zero** path invocations") classifies them
   correctly as entrypoints — but only because it requires *zero* path invocations.
   This validates the AC1 wording and is the reason a naive heuristic fails.

4. **The guard slots in with a known 3-point registration** and no `__init__.py`
   change: a new `@task` in `tasks/lint/scripts.py`, a `[tasks."lint:scripts:<name>:check"]`
   block in `mise.toml`, and an entry in the `lint:scripts:check` `depends` list.

5. **0107 will almost certainly land second.** It is `draft` with no plan; 0122 is
   `ready`/approved. The "shared extension point" rebase risk is real but minimal —
   expect 0122 to define the wiring 0107 rebases against.

6. **Documentation home: `tasks/README.md` "Conventions (learn once)"** (lines
   30-45) is where the shell-lint conventions already live (incl. "shell has no
   autofixer"); `CLAUDE.md`'s shell section is the secondary home.

## Detailed Findings

### Area 1 — Live executable-bit audit (the ground truth)

A `stat` sweep of all tracked `.sh` (excluding `node_modules/`, `workspaces/`,
`target/`) cross-referenced with call-site classification yields these violations of
the *executable ⟺ off-the-library-list* invariant:

**Entrypoints missing `+x` (`644` today → must be `755`)** — 18 files:

| Script | Why it is an entrypoint |
| --- | --- |
| `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh` | Run via `bash "$f"` (`run-migrations.sh:273`); siblings 0001–0007 are `755`. *(named in WI)* |
| `scripts/test-interactive-protocol.sh` | Test runner; discovered by exec-bit in `tasks/test/helpers.py:38`. *(named in WI)* |
| `skills/integrations/jira/scripts/jira-attach-flow.sh` | bare-path in `attach-jira-issue/SKILL.md:40,98` |
| `skills/integrations/jira/scripts/jira-comment-flow.sh` | bare-path in `comment-jira-issue/SKILL.md:82,90,155,163` |
| `skills/integrations/jira/scripts/jira-create-flow.sh` | bare-path in `create-jira-issue/SKILL.md:177,235`; `jira-emit-key.sh:27` |
| `skills/integrations/jira/scripts/jira-search-flow.sh` | bare-path `search-jira-issues/SKILL.md:71`; `work-item-fetch-remote.sh:120,258` |
| `skills/integrations/jira/scripts/jira-transition-flow.sh` | bare-path in `transition-jira-issue/SKILL.md:65,148` |
| `skills/integrations/jira/scripts/jira-update-flow.sh` | bare-path `update-jira-issue/SKILL.md:76,145`; `work-item-update-remote.sh:143` |
| `skills/work/scripts/work-item-fetch-remote.sh` | `bash` from `work-item-sync-apply.sh:122`; `sync-work-items/SKILL.md:95,275` |
| `skills/work/scripts/work-item-file-dirty.sh` | `bash "$FILE_DIRTY"` in `test-work-item-scripts.sh:1568,1586` |
| `skills/work/scripts/work-item-normalise.sh` | `bash` from `work-item-sync-apply.sh:128,130,174,175` et al. |
| `skills/work/scripts/work-item-project-remote.sh` | `bash` from `work-item-sync-apply.sh:125,127` |
| `skills/work/scripts/work-item-section-diff.sh` | bare-path `sync-work-items/SKILL.md:200` |
| `skills/work/scripts/work-item-sync-apply.sh` | bare-path `sync-work-items/SKILL.md:326` |
| `skills/work/scripts/work-item-sync-baseline.sh` | `bash` from `work-item-sync-apply.sh:133,176,193`; `list-work-items/SKILL.md:283` |
| `skills/work/scripts/work-item-sync-classify.sh` | bare-path `sync-work-items/SKILL.md:124`; `list-work-items/SKILL.md:337` |
| `skills/work/scripts/work-item-sync-decide.sh` | bare-path `sync-work-items/SKILL.md:66,142,220` |
| `skills/work/scripts/work-item-update-remote.sh` | `bash` from `work-item-sync-apply.sh:110` |

**Libraries carrying `+x` (`755` today → must be `644`)** — 5 files:

| Script | Source-refs | Notes |
| --- | --- | --- |
| `scripts/atomic-common.sh` | 15+ (`config-common.sh:10`, `run-migrations.sh:9`, …) | *(named in WI)* |
| `scripts/config-common.sh` | ~40 (config-*.sh, hooks/vcs-*, …) | *(named in WI)* |
| `scripts/vcs-common.sh` | ~10 (`hooks/vcs-detect.sh`, `launch-server.sh`, …) | *(named in WI)* |
| `skills/work/scripts/work-item-common.sh` | ~8 (`work-item-*.sh` family) | *(named in WI)* |
| `skills/visualisation/visualise/scripts/test-helpers.sh` | `test-stop-server.sh:7`, `test-status-server.sh:7`, `test-launch-server.sh:7` | **NOT named in WI** — sourced-only yet `755`; contradicts `tasks/test/helpers.py:20-22` |

Both the WI seed-set libraries already at `644` (`fs-common.sh`, `hash-common.sh`,
`jsonl-common.sh`, `log-common.sh`, `work-common.sh`, `config-defaults.sh`,
`doc-type-table.sh`, `frontmatter-emission-rules.sh`, `frontmatter-fixtures.sh`,
`interactive-harness.sh`, `interactive-protocol.sh`, `test-helpers.sh`,
`interactive-lib.sh`, `accelerator-scaffold.sh`, `doc-type-inference.sh`) are
correct as-is and need no mode change — only listing.

### Area 2 — The library-list contents (what the manifest must hold)

Compiled from the verified audit, the **complete** sourced-only set (every one
confirmed: ≥1 `source`/`.` reference, **zero** path invocations) is **26 files**:

- `scripts/`: `fs-common.sh`, `hash-common.sh`, `jsonl-common.sh`, `log-common.sh`,
  `work-common.sh`, `config-defaults.sh`, `config-common.sh`, `atomic-common.sh`,
  `vcs-common.sh`, `doc-type-table.sh`, `doc-type-inference.sh`,
  `frontmatter-emission-rules.sh`, `frontmatter-fixtures.sh`,
  `interactive-harness.sh`, `interactive-protocol.sh`, `test-helpers.sh`,
  `accelerator-scaffold.sh`
- `skills/config/migrate/scripts/interactive-lib.sh`
- `skills/github/scripts/test-helpers.sh`
- `skills/visualisation/visualise/scripts/`: `launcher-helpers.sh`, `test-helpers.sh`
- `skills/work/scripts/`: `work-item-common.sh`, `work-item-bridge-codes.sh`
- `skills/integrations/jira/scripts/`: `jira-common.sh`, `jira-auth.sh`, `jira-jql.sh`,
  `jira-body-input.sh`, `jira-custom-fields.sh`
- `skills/integrations/linear/scripts/`: `linear-common.sh`, `linear-auth.sh`

> Note this is **larger than the WI's 15-item seed set** — the sweep added
> `launcher-helpers.sh`, `work-item-bridge-codes.sh`, the jira/linear `*-common.sh`
> /`*-auth.sh`, `jira-jql.sh`, `jira-body-input.sh`, `jira-custom-fields.sh`, plus
> `config-common.sh`/`atomic-common.sh`/`vcs-common.sh`/`work-item-common.sh` (the
> "+x" libs) and both extra `test-helpers.sh`.

There is also `skills/visualisation/visualise/cli/accelerator-visualiser`
(extensionless) which `shell_sources()` explicitly appends via
`_EXTRA_SHELL_SOURCES` (`tasks/shared/sources.py:55-57`) — the guard sees it, so its
classification (it is an entrypoint, currently executable) must be considered too.

### Area 3 — The dual-use trap (validates AC1 wording)

Three scripts are `source`d (by tests) **and** invoked by path (in production), so
"sourced ⇒ library" alone would wrongly strip their `+x`:

- `scripts/linkage-parser.sh` — sourced by `test-linkage-parser.sh:14`, but run
  `bash "$PARSER" "$f"` at migration `0007:624`; header self-declares "runnable as a
  CLI" (`linkage-parser.sh:16`). Currently `755` (correct).
- `skills/design/inventory-design/scripts/validate-source.sh` — sourced by
  `test-validate-source.sh:17`, but invoked by path in `inventory-design/SKILL.md:56`.
  Currently `755` (correct).
- `skills/integrations/jira/scripts/jira-fields.sh` — sourced by `jira-init-flow.sh:32`,
  but invoked `bash …/jira-fields.sh resolve|refresh|list` at `jira-search-flow.sh:96`,
  `jira-update-flow.sh:303`, `jira-create-flow.sh:284`, `jira-init-flow.sh:262,268`.
  Currently `755` (correct).

The WI's AC1 ("a repo-wide search finds ≥1 `source` reference **and zero** bare-path
or `bash/sh/env`-prefixed invocations") classifies all three correctly as
entrypoints, **because of the "zero path invocations" half**. Anyone building the
list must apply both halves, not just "is it sourced anywhere".

### Area 4 — Migration test-fixtures (the unhandled population)

Eight `644` files under `skills/config/migrate/scripts/test-fixtures/**` (the seven
`interactive/*/migrations/*.sh` plus `baseline-no-pending/seed.sh`):

- The migration runner discovers files by **name**, not mode:
  `find "$MIGRATIONS_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.sh'`
  (`run-migrations.sh:163`) — no `-perm`/`-x` test — and executes via `bash "$f"`
  (`run-migrations.sh:273`, `interactive-lib.sh:357`). `seed.sh` is run
  `bash "$seed"` (`test-migrate-snapshot.sh:79`).
- So they are *functionally* fine at `644` (never need `+x`, never sourced), but
  `shell_sources()` **keeps fixtures** (`tasks/shared/sources.py:_keep` only
  excludes `workspaces/`; `tests/unit/tasks/shared/test_sources.py:28-41` asserts
  fixtures are kept). The guard *will* enumerate them.

**Implication for 0122**: the bidirectional guard as written would flag all eight as
"entrypoint missing `+x`". The work item does not address this. Options: (a) add the
eight fixtures to the library-list (semantically odd — they are not libraries, they
are bash-run fixtures), (b) give the guard a `test-fixtures/` path exemption, or
(c) extend `shell_sources()`/the guard to skip fixtures. This is a **decision the
plan must make**; the cleanest is probably a narrow fixture-path exemption inside the
guard, leaving `shell_sources()` untouched.

### Area 5 — Where the guard lives: `tasks/lint/scripts.py`

`tasks/lint/scripts.py` (48 lines) holds two `@task`s — `shellcheck` and `bashisms`
— each following one idiom (`tasks/lint/scripts.py:13-47`):

```python
def _sources_args() -> str | None:
    sources = shell_sources()
    if not sources:
        return None
    return " ".join(shlex.quote(s) for s in sources)

@task
def bashisms(context: Context) -> None:
    args = _sources_args()
    if args is None:
        raise Exit(f"bashisms: {_EMPTY_SCOPE}", code=1)
    with context.cd(str(repo_root())):
        result = context.run(f"bash scripts/lint-bashisms.sh {args}", warn=True, pty=False)
    if result.exited != 0:
        raise Exit("lint-bashisms found bash-4 constructs", code=1)
```

Patterns the new guard must mirror: enumerate via `shell_sources()` (never
per-task globbing); **fail-closed on empty scope** (`_EMPTY_SCOPE`); raise a single
`invoke.Exit(message, code=1)` listing offenders. There is **no shared "run a tool
over `shell_sources()`" wrapper** — `tasks/format/scripts.py` duplicates the same
boilerplate. A *pure-Python* guard (no external scanner) is the natural fit here
(it `os.stat`s modes), so it would build its own offender list and
`raise Exit("\n".join(offenders), code=1)`.

### Area 6 — `shell_sources()` enumeration (`tasks/shared/sources.py:60`)

`shell_sources(root=None)` (`tasks/shared/sources.py:60-100`) does an `os.walk` from
`repo_root()`, pruning gitignored directories in place (`dirnames[:] = …`), skipping
gitignored files, keeping only `*.sh`, dropping `workspaces/` via `_keep`
(`:29-37`), then appending `_EXTRA_SHELL_SOURCES` (the extensionless CLI). It honours
**only the root `.gitignore`** (built into a `pathspec.GitIgnoreSpec`, plus `.git/`
and `.jj/`, `:40-48`). It returns a **sorted `list[str]` of repo-relative POSIX
paths**, and takes a `root: Path | None` test seam.

This is exactly why AC5's spot-check passes: `node_modules/` is gitignored and pruned,
so `…/playwright-core/bin/reinstall_chrome_stable_linux.sh` never enters the set;
`workspaces/`/`target/` are likewise excluded. Reusing it (mandatory per the WI)
means the guard inherits correct exclusions for free.

> **One subtlety**: `shell_sources()` returns *working-tree* files honouring
> `.gitignore`, not VCS-recorded modes. The guard reads the **working-copy** mode
> (`os.stat`/`os.access(..., os.X_OK)`), while the WI's acceptance criteria speak of
> "committed mode recorded by the VCS (as seen on a fresh git clone)". In practice
> these agree because the exec bit is a tracked attribute and a committed `chmod`
> propagates — but the guard tests the working copy, so a local uncommitted `chmod`
> would pass/fail the guard before it is committed. Worth a one-line note in the plan.

### Area 7 — Wiring into `mise run check` (the 3-point registration)

1. Add `@task def <guard>(context)` to `tasks/lint/scripts.py`. It auto-registers as
   `invoke lint.scripts.<guard>` via `Collection.from_module(lint.scripts)` in
   `tasks/__init__.py:71` — **no `__init__.py` edit needed** (the module is already
   imported in `tasks/lint/__init__.py`).
2. Add a `[tasks."lint:scripts:<guard>:check"]` block to `mise.toml` running
   `invoke lint.scripts.<guard>`, and append it to the `lint:scripts:check`
   `depends` list (`mise.toml:236`).
3. `lint:scripts:check` (`mise.toml:234`) → `scripts:check` (`:238`) → `check`
   (`:351`); the default task runs `lint:check` (`:334`) which also depends on
   `lint:scripts:check`. CI runs `mise run scripts:check` (`.github/workflows/main.yml:99-115`).

Report-only / no-autofixer is automatically satisfied: `scripts` is deliberately
absent from `lint:fix` (`mise.toml:339-340`), so adding nothing there preserves the
"shell has no autofixer" convention the WI requires.

### Area 8 — Test patterns the guard's tests should follow

`tests/unit/tasks/test_lint.py` is the closest template, with two layers:

- **Layer A — mocked `Context`** (`test_lint.py:14-72`): patch the source list with
  `mocker.patch.object(lint, "shell_sources", return_value=[...])` (it is imported
  into the lint module's namespace), assert command construction, `pytest.raises(Exit)`
  on findings, and **fail-closed on empty scope** with `ctx.run.assert_not_called()`.
- **Layer B — behavioural** (`test_lint.py:75-158`): run the real check over a
  `tmp_path` tree via `subprocess.run`, assert `returncode` and the offender label in
  output.

Exec-bit manipulation has direct prior art in `tests/unit/tasks/test_integration.py:27-43`:
`p.chmod(0o755)` / `p.chmod(0o644)` plus `monkeypatch.setattr(helpers, "repo_root",
lambda: tmp_path)`. Production reads the bit with `os.access(p, os.X_OK)`
(`tasks/test/helpers.py:38`). Enumeration tests with exact sorted-list assertions and
a `root=tmp_path` seam live in `tests/unit/tasks/shared/test_sources.py`. The
"vacuous-pass / can't-silently-shrink" guard pattern (a sentinel probe) is modelled by
`tests/unit/tasks/test_python_coverage.py:133-176` — directly relevant to proving the
new guard cannot pass vacuously.

Conventions (`pyproject.toml`): importlib mode + `pythonpath=["."]` (`:40-42`); no
`__init__.py` in tests; tests excluded from pyrefly (`:124-146`) and relaxed in ruff
(`:106`, incl. `SLF001` so private symbols like `_keep` are testable).

## Code References

- `tasks/lint/scripts.py:13-47` — the `_sources_args()` + fail-closed `@task` idiom to mirror
- `tasks/shared/sources.py:60-100` — `shell_sources()` (mandated enumeration); `:29-37` `_keep`; `:55-57` `_EXTRA_SHELL_SOURCES`
- `tasks/__init__.py:69-85` — `Collection.from_module(lint.scripts)` auto-registration
- `mise.toml:226-240` — `lint:scripts:*` → `lint:scripts:check` → `scripts:check`; `:334-356` `lint:check`/`check`/`default`; `:339-340` `lint:fix` excludes scripts
- `.github/workflows/main.yml:99-115` — `check-scripts` runs `mise run scripts:check`
- `tasks/test/helpers.py:17-44` — `run_shell_suites` exec-bit discovery (`os.access(..., os.X_OK)`, `:38`); `:20-22,37` the `test-helpers.sh` name-exclusion + "must NOT be executable" comment
- `skills/config/migrate/scripts/run-migrations.sh:163,273` — migration discovery by `find -name` + `bash "$f"` (fixtures need no `+x`)
- `tests/unit/tasks/test_lint.py:14-158` — mocked + behavioural test template
- `tests/unit/tasks/test_integration.py:27-43` — `chmod`/exec-bit fixture precedent
- `tests/unit/tasks/shared/test_sources.py:28-67` — enumeration test template (fixtures kept)
- `tests/unit/tasks/test_python_coverage.py:133-176` — anti-vacuous-pass sentinel-probe template

## Architecture Insights

- **The exec bit is load-bearing under 0106, but only one mechanism strictly
  requires it.** SKILL.md bodies invoke by bare path (so an entrypoint without `+x`
  fails / escapes `allowed-tools`), but sibling-to-sibling and migration invocations
  use `bash X` (which never stat-checks the bit). The single mechanism that *requires*
  `+x` is the Python suite runner (`tasks/test/helpers.py:38`). The codebase
  convention — evidenced by the `755` jira/linear flow siblings — is to mark every
  shebang-bearing, bare-path/`bash`-invoked entrypoint `755` regardless. The WI
  correctly adopts "consistency 755" rather than "strictly-required 755".
- **The library-list fails safe toward executable**, which is right: entrypoints are
  the majority and a newly-added unclassified script defaults to "must be `+x`".
- **The bidirectional invariant is only sound because AC1's membership rule is
  strict** ("sourced AND never invoked by path"). The dual-use scripts show the
  "never invoked by path" half is doing real work.
- **`shell_sources()` keeping fixtures** is a deliberate 0098 widening — good for
  shellcheck/shfmt coverage, but it means the new guard inherits the fixture
  population and must decide what to do with it.

## Historical Context

- `meta/work/0106-invoke-plugin-scripts-by-bare-path.md` (**done**, validated) —
  establishes "scripts self-execute (shebang + execute bit)" and "invoke by bare
  path, never `bash`/`sh`/`env` (which escapes `allowed-tools`)". The reason the
  exec bit matters. No `allowed-tools` changes were made; it relies entirely on the
  shebang + exec bit.
- `meta/work/0098-repo-wide-linting-formatting-static-analysis.md` (**done**) +
  `meta/plans/2026-06-10-0098-…md` — built `lint:scripts:*` (shellcheck/bashisms/shfmt),
  `.shellcheckrc`, the bash-3.2 bashisms linter, the `scripts` check component, and
  `shell_sources()` (the jj-workspace-safe walk that replaced `git ls-files`).
- `meta/work/0107-lint-skill-body-script-invocations.md` (**draft, no plan,
  blocked_by 0106**) — companion lint sharing the `tasks/lint/scripts.py` extension
  point. 0122 will land first; 0107 rebases against 0122's wiring.
- `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
  — origin research for the bare-path convention (the "why" behind 0106/0107/0122).
- `tasks/README.md:30-45` ("Conventions — learn once") and `CLAUDE.md` shell section —
  where shell-lint conventions are documented today (incl. "shell has no autofixer").
  The library-list mechanism should be documented in `tasks/README.md` per AC6.
- No prior research/plan/decision addresses executable bits / file modes — 0122 is
  the first.

## Related Research

- `meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`
  — enumerates the `artifact-*`/`config-*` bare-path call sites for 0106.
- `meta/research/codebase/2026-06-20-0116-structured-stall-on-no-decision-input.md`
  — concurrent research (unrelated topic; same date).

## Open Questions

1. **Fixture population** (Area 4): how should the guard treat the eight
   `test-fixtures/**` `.sh` files `shell_sources()` keeps but that never need `+x`?
   Recommend a narrow `test-fixtures/` path exemption in the guard (not library-list
   entries, not flipping to `755`). The plan must decide.
2. **Working-copy vs VCS-recorded mode** (Area 6): the guard reads the working-copy
   mode via `os.stat`/`os.access`, while the WI's ACs speak of committed VCS mode.
   They agree in practice; confirm the plan tests/words this consistently.
3. **Scope acknowledgement**: the WI names 6 corrections; the real set is ~23 (18
   entrypoints + 5 libraries). The WI says it would raise priority to *high* if step-1
   finds a *currently-shipped* entrypoint broken — the 6 jira flows and the
   work-item sync scripts **are** packaged, bare-path-invoked entrypoints (the strict
   0106 case), so the escalation trigger appears to fire. Confirm with the
   release/packaging owner whether priority should rise.
4. **`accelerator-visualiser`** (extensionless, in `_EXTRA_SHELL_SOURCES`): confirm
   its classification (entrypoint, executable) and that the guard handles the
   extensionless extra correctly.
