---
type: "codebase-research"
id: "2026-09-06-0245-invoke-accelerator-directly-in-skills"
title: "Research: Invoke Accelerator Directly In Skills (0245)"
date: "2026-09-06T10:02:32+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0245"
parent: "work-item:0245"
relates_to: ["codebase-research:2026-07-27-0182-plugin-root-self-location-implementation-surface", "codebase-research:2026-07-19-0167-config-command-and-invocation-contract-migration", "codebase-research:2026-08-19-0212-work-item-script-cutover", "codebase-research:2026-06-11-0106-bare-path-script-invocation-call-sites"]
topic: "Converting SKILL.md bodies to bare accelerator invocation"
tags: ["research", "codebase", "skills", "cli", "lint", "plugin-root", "path"]
revision: "d42d4a4c08f02d33c1aecb62462c7151b6769686"
repository: "accelerator"
last_updated: "2026-09-06T10:02:32+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Corrected the PATH-precondition finding: plugin bin/ is auto-added to PATH per the Claude Code plugins reference, so the bare form resolves; the earlier unmet conclusion was a false inference from repo absence"
schema_version: 1
---

# Research: Invoke Accelerator Directly In Skills (0245)

**Date**: 2026-09-06T10:02:32+00:00
**Author**: Toby Clemson
**Git Commit**: d42d4a4c08f02d33c1aecb62462c7151b6769686
**Branch**: visualisation-system (jj workspace)
**Repository**: accelerator

## Research Question

What does the codebase need to converge every SKILL.md body onto bare
`accelerator …` invocation (dropping the `${CLAUDE_PLUGIN_ROOT}/bin/` prefix),
per work item 0245 — the current invocation forms, the lint surface that would
guard the convention, and whether the gating PATH / `CLAUDE_PLUGIN_ROOT`
precondition actually holds?

## Summary

**0245 is viable — the gating PATH precondition holds — but it is a
three-surface co-migration plus a lint reconciliation, not the find-replace the
item describes.** Corrected after review: the Claude Code plugins reference
states a plugin's `bin/` is "added to the Bash tool's `PATH` and invokable as
bare commands while the plugin is enabled", so bare `accelerator` resolves by
platform convention. The earlier "precondition unmet" reading was wrong — it
inferred absence from the repo carrying no PATH declaration, but PATH placement
is Claude Code's behaviour, not something the plugin declares. Three findings
shape the actual work:

- ✅ **PATH precondition satisfied for the Bash tool.** Plugin `bin/` is
  auto-added to PATH; `CLAUDE_PLUGIN_ROOT` need not be in the environment because
  bare `accelerator` resolves via PATH and the bootstrap self-locates. One
  residual: the docs name "the Bash tool's PATH" — a single smoke-test confirms
  the `!` preprocessor shares it.
- ⚠️ **Three surfaces move together, not "bodies only".** The prefix lives in `!`
  body calls *and* frontmatter `Bash(...)` grants; both must go bare in lockstep
  or the permission matcher prompts. `skill_permissions.py` / `skill_parsing.py`
  actively expect the prefix, so the existing lint layer is rewritten, not merely
  extended.
- ⚠️ **Distribution caveat.** The same reference says a top-level `bin/` cannot
  ship in a plugin distributed through claude.ai organization settings. If epic
  0136's distribution ever targets that channel, the bare-command mechanism is
  unavailable there — confirm the distribution path before committing to bare as
  the sole form.

The item's claim that 0167/0212 already produced bare call sites is still
imprecise (they kept the prefix), but that no longer blocks anything — it just
means 0245 is the first cluster to go bare, with no in-tree precedent to copy.

## Detailed Findings

### Current invocation forms across skills

The tree is uniform on the **prefix form** (`${CLAUDE_PLUGIN_ROOT}/bin/accelerator …`).

| Metric | Value |
| --- | --- |
| Total `SKILL.md` under `skills/` | 68 |
| Files referencing `accelerator` | 48 |
| Files with a live prefixed invocation | 47 |
| `/bin/accelerator` occurrences | 427 |
| Absolute-path calls | 0 |
| `bash`-wrapper calls | 0 |
| Bare executable calls | 0 |

Every one of the 427 occurrences matches `CLAUDE_PLUGIN_ROOT.*/bin/accelerator`
— there are no fully-expanded absolute paths and no `bash`/`sh`/`env` wrappers.
The only bare `accelerator` strings are **documentation prose** inside inline
backticks or fenced terminal examples (e.g.
`skills/review/output-formats/work-item-review-output-format/SKILL.md:63`,
`skills/config/migrate/SKILL.md:191`), never a `!` preprocessor call.

⚠️ **Two surfaces carry the prefix, not one.** Each file places it both in the
body `!` preprocessor lines *and* in the frontmatter `allowed-tools` grants:

- Body call — `skills/visualisation/visualise/SKILL.md:13`:
  ``!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill visualise --fail-safe` ``
- Frontmatter grant — `skills/visualisation/visualise/SKILL.md:7`:
  `- Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)`

The item scopes itself to "SKILL.md bodies". That is a definition trap: if
bodies go bare while the `Bash(...)` grant stays prefixed, the bare call is not
covered by the grant and the permission matcher prompts — the exact regression
0106 was created to kill. The 47 files span every cluster (config, work,
planning, research, decisions, design, github, and the Jira/Linear integrations).

### The PATH / `CLAUDE_PLUGIN_ROOT` precondition — satisfied by platform convention

This is the load-bearing correction. Plugin `bin/` is placed on PATH by Claude
Code itself, so bare `accelerator` resolves. The Claude Code plugins reference is
explicit: executables in `bin/` are "added to the Bash tool's `PATH` and
invokable as bare commands while the plugin is enabled". PATH placement is a
platform behaviour, not a plugin declaration — which is why searching the repo
(`plugin.json`, hooks) for a PATH-injection mechanism finds nothing yet is not
evidence against it. The earlier codebase-only research drew exactly that false
inference.

The two in-repo facts remain true, but they are compatible with bare invocation,
not counter to it:

1. **No manifest injects PATH — as expected.** `.claude-plugin/plugin.json` has
   no `bin`/`env`/`PATH` field because the plugin does not need one; Claude Code
   adds `bin/` automatically. The manual terminal symlink chain
   (`docs-site/src/content/docs/internals.md:145-231`) is a separate ergonomic
   for invoking `accelerator` from an ordinary terminal *outside* Claude Code, not
   the mechanism skills rely on.

2. **`CLAUDE_PLUGIN_ROOT` is not in the skill-shell environment — and does not
   need to be.** The 0182 probe (Claude Code v2.1.220,
   `meta/research/issues/2026-07-26-cli-requires-claude-plugin-root-env-var.md`)
   confirmed Claude Code substitutes `${CLAUDE_PLUGIN_ROOT}` textually and exports
   it as a real variable only to hooks and MCP/LSP subprocesses. Bare
   `accelerator` sidesteps this entirely — it resolves via PATH, then the
   bootstrap self-locates its root and exports `ACCELERATOR_PLUGIN_ROOT`
   (`bin/accelerator:93-120`). The prefix form and the bare form both terminate at
   the same self-locating bootstrap; only the *lookup* differs — substituted
   absolute path versus PATH resolution.

⚠️ **Distribution caveat.** The plugins reference warns that a top-level `bin/`
cannot be included in a plugin distributed through claude.ai organization
settings (its "keep executables out of the top-level `bin` directory" guidance).
Bare invocation depends on that on-PATH `bin/`; if epic 0136's distribution ever
moves to that channel, the mechanism is unavailable there. Confirm the
distribution path before committing to bare as the sole form.

Residual to close empirically: the docs name "the Bash tool's PATH". A SKILL.md
`!` preprocessor is a distinct execution surface the docs do not name; a one-line
smoke-test — a bare `` !`accelerator …` `` in a throwaway skill, run in both a
main session and a subagent — confirms it shares that PATH. The item's Open
Question is answered "yes" for the Bash tool by the docs; this closes the
`!`-context corner.

No bare-form precedent exists in-tree yet: a grep for a bare
`` !`accelerator … `` preprocessor across `skills/` returns zero matches, and the
0167 plan kept the prefix verbatim (`meta/plans/2026-07-19-0167-…-migration.md:142-145`).
That is a "not done yet", not a "cannot" — 0245 is simply the first cluster to
convert.

### Lint surface — reuse a template, but rewrite existing enforcement

The guarding lint has a near-exact template and a namespaced home, but the item
under-counts the work: existing lints already encode the *opposite* convention.

- ✅ **Template exists.** `tasks/lint/call_site_migration.py` scans
  `skills/**/SKILL.md` line-by-line for a forbidden substring and raises
  `invoke.Exit(msg, code=1)` on findings — the exact shape a bare-form allowlist
  needs. `tasks/lint/scripts.py` (bashisms) is the canonical bespoke-content-lint
  pattern: a pure `scan_*(...) -> list[str]` helper separated from a thin `@task`.
- ⚠️ **Existing lints expect the prefix.** `tasks/shared/skill_parsing.py`
  defines `PLUGIN_PREFIX` / `LAUNCHER` (and, notably, an already-present
  `BARE_LAUNCHER`) constants; `tasks/lint/skill_permissions.py` cross-checks each
  body invocation against its `allowed-tools` grant assuming the pathed launcher.
  Flipping to the bare form means rewriting these, not layering a new rule on top.
- ⚠️ **`check` does not run the skill lints today.** `lint:skill-permissions` and
  `lint:call-site-migration` appear only under `lint:check`, which the fast
  `mise run check` does *not* depend on — only the full `default` run reaches
  them (`mise.toml:646-668`). A new `lint:skills:check` must be registered in a
  component roll-up `check` depends on (natural home: `build-system:check`,
  `mise.toml:506`) *and* in `lint:check` to be covered in every lane.

Wiring a new lint is a five-step mechanical path (module under `tasks/lint/`,
export in `tasks/lint/__init__.py`, collection in `tasks/__init__.py`, a
`mise.toml` façade with `depends = ["deps:install:python"]`, and roll-up
registration), with a TDD pytest module under `tests/unit/tasks/`.

### Relationship to 0107 and the acceptance criteria

0245 owns and supersedes 0107 ("Lint Skill-Body Script Invocations",
`meta/work/0107-…md`, status `draft`, `blocked_by: [0106]`). 0107's intended
check was a per-SKILL.md shape-vs-`allowed-tools` coverage guard (POSIX `awk`, no
PCRE2, known-positive seed). Its scope is subsumed here; close or re-point it
rather than build a duplicate. The 0245 review
(`meta/reviews/work/0245-…-review-1.md`) already flagged three gaps that this
research confirms: the lint-ownership overlap with 0107 is unrecorded in
Dependencies; only one of three forbidden forms has a grep AC (absolute-path and
`bash`-wrapper checks are missing); and AC4 ("no invocation regressions") is
tautological because the PATH precondition is untested by any criterion.

## Code References

- `skills/visualisation/visualise/SKILL.md:7,13` - prefix form in both the
  `allowed-tools` grant and the `!` preprocessor body call
- `bin/accelerator:93-120` - bootstrap self-locates root and exports
  `ACCELERATOR_PLUGIN_ROOT`; agnostic to how it is found
- `hooks/hooks.json:9,18,27,47` - hooks dispatch by absolute placeholder path
- `.claude-plugin/plugin.json:10-26` - no `bin`/`PATH`/`env` declaration needed;
  Claude Code auto-adds plugin `bin/` to PATH (plugins reference)
- `docs-site/src/content/docs/internals.md:145-231` - manual terminal `PATH`
  symlink chain (opt-in, outside skill contexts)
- `tasks/lint/call_site_migration.py:26-82` - template: scan `skills/**/SKILL.md`,
  `Exit(code=1)` on findings
- `tasks/lint/scripts.py:75-109` - pure `scan_*` helper + thin `@task` pattern
- `tasks/lint/skill_permissions.py:149-172` - existing body-vs-grant coverage lint
- `tasks/shared/skill_parsing.py:23-28` - `PLUGIN_PREFIX`/`LAUNCHER`/`BARE_LAUNCHER`
- `mise.toml:506,646-668` - `build-system:check` roll-up and the `check`/`default`
  lint-coverage gap

## Architecture Insights

- **Two resolution paths, one bootstrap.** The prefix form resolves by textual
  path substitution; the bare form resolves via the auto-provisioned on-PATH
  `bin/`. Both then reach the same self-locating bootstrap, so they are
  behaviourally identical once found — the item's "no behavioural change" claim
  holds by construction.
- **Body and frontmatter are one unit.** The permission matcher couples the `!`
  preprocessor call to its `allowed-tools` grant; 0167 migrated them "in
  lockstep" for exactly this reason. Any 0245 conversion must move both.
- **`BARE_LAUNCHER` already exists in `skill_parsing.py`.** The bare form was
  anticipated in the shared parsing layer — worth reading before designing new
  detection, since partial scaffolding may already be present.

## Historical Context

- `meta/work/0106-invoke-plugin-scripts-by-bare-path.md` (done) - dropped the
  `bash` wrapper only, kept the braced path; driven by the permission matcher
  stripping only `timeout`/`nice`/etc., not `bash`.
- `meta/work/0167-…migration.md` (done) & `meta/plans/2026-07-19-0167-…md:142-145`
  - config cluster moved script→`bin/accelerator config`, **prefix retained**.
- `meta/work/0212-work-item-script-cutover.md` (done) - work cluster
  script→`bin/accelerator work`, prefix retained.
- `meta/work/0182-cli-derives-plugin-root-from-own-location.md` (done) &
  `meta/research/issues/2026-07-26-cli-requires-claude-plugin-root-env-var.md` -
  the definitive env-contract evidence; established the invariant "no plugin
  entry point may require `CLAUDE_PLUGIN_ROOT` from its process environment".
- `meta/reviews/work/0245-…-review-1.md` - prior review flagging the lint-vs-0107
  overlap, the absolute-rule/fallback contradiction, and the untested PATH AC.

## Related Research

- `meta/research/codebase/2026-07-27-0182-plugin-root-self-location-implementation-surface.md`
- `meta/research/codebase/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
- `meta/research/codebase/2026-08-19-0212-work-item-script-cutover.md`
- `meta/research/codebase/2026-06-11-0106-bare-path-script-invocation-call-sites.md`

## Open Questions

- ❓ **Does the `!` preprocessor share the Bash tool's PATH?** The plugins
  reference confirms plugin `bin/` is on "the Bash tool's PATH"; the `!`
  preprocessor is a distinct execution surface the docs do not name. A one-line
  smoke-test (bare `` !`accelerator …` `` in a throwaway skill, main session and
  subagent) closes it. Presumed yes.
- ❓ **Is org-settings distribution in scope for epic 0136?** A top-level `bin/`
  cannot ship through claude.ai organization settings; if that channel is
  targeted, bare invocation is unavailable there and the prefix form (or a
  relocated executable) would be needed.
- ❓ **Are frontmatter `Bash(...)` grants in scope?** They must change with the
  bodies for permissions to hold; the item's "bodies only" scope should be
  restated to include the paired grants.
- ❓ **Does bare `accelerator` collide with any other `accelerator` on `PATH`?**
  A bare name resolves to the first match; the prefixed path cannot. Worth
  confirming no ambiguity if a PATH mechanism is introduced.
