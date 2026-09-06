---
type: "plan"
id: "2026-09-06-0245-invoke-accelerator-directly-in-skills"
title: "Invoke Accelerator Directly In Skills Implementation Plan"
date: "2026-09-06T12:18:47+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0245"
parent: "work-item:0245"
derived_from: ["codebase-research:2026-09-06-0245-invoke-accelerator-directly-in-skills"]
tags: ["skills", "cli", "lint"]
revision: "0bf2ab6cb58936ce319804389eab606d75e8f942"
repository: "accelerator"
last_updated: "2026-09-06T16:16:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Invoke Accelerator Directly In Skills Implementation Plan

## Overview

Converge every `SKILL.md` onto bare `accelerator …` invocation, dropping the
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator` prefix, and guard the convention with a
new allowlist lint wired into the fast `check` lane. The prefix is redundant —
Claude Code auto-adds the plugin `bin/` to PATH — and one uniform call form is
easier to allowlist, read, and maintain.

## Current State Analysis

The tree is uniform on the prefix form: **427 occurrences of
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator` across 47 of 68 `SKILL.md` files**, with
no absolute-path or `bash`-wrapper variants. Those 427 live in three surfaces,
all of which must convert for the acceptance-criteria greps to reach zero:

| Surface | Count | Role |
|---|---|---|
| Body `` !`…` `` preprocessor calls | 210 | Live invocations, run at skill load |
| Frontmatter `Bash(…)` grants | 97 | Permission cover for the body calls |
| Prose + fenced-block examples | 120 | Documentation and numbered-step commands |

The substitution is total and line-local: every `/bin/accelerator` occurrence is
the `${CLAUDE_PLUGIN_ROOT}`-prefixed form (427 = 427), so replacing
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator` with `accelerator` cannot mangle an
unrelated `bin/accelerator` reference — there are none.

The `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` convention is not confined to two
lint modules. A repo-wide grep for `/bin/accelerator`, `PLUGIN_PREFIX`,
`LAUNCHER`, and `is_plugin_invocation` finds it independently encoded across
four production/support modules and four test files, every one of which breaks
— several **silently** — on conversion. The full coupled surface:

**Production / support code:**

- `tasks/shared/skill_parsing.py:24,28,91-93` — `LAUNCHER` / `BARE_LAUNCHER`
  embed the prefix, and `is_plugin_invocation()` returns
  `command.startswith("${CLAUDE_PLUGIN_ROOT}/")`. A bare `accelerator …` command
  returns `False`, so every converted call drops out of the coverage check and
  the injection census (`EXPECTED_INJECTION_SKILLS = 42`) collapses to 0.
- `tasks/lint/skill_permissions.py:50-53` — `_CONFIG_MARKER`, `_CONTEXT_SKILL`,
  `_CONTEXT_ANY`, `_INSTRUCTIONS` all key on the `/bin/accelerator` substring;
  the `--fail-safe`, context, and instructions checks match nothing after
  conversion.
- `tasks/shared/skill_write_gate.py:32` — the write-gate mutation matcher is
  `rf"/bin/accelerator {provider} (?:{verbs})\b"`. After conversion it matches
  no write skill, so `_mutation_offset` returns `None` everywhere, the
  anti-vacuity guard fires, and `lint:integration-skills:check` — a dependency
  of `build-system:check` (`mise.toml:506`), itself a dependency of `check`
  (`mise.toml:664`) — exits non-zero. **This turns `mise run check` red.**
- `tests/integration/support/skill_corpus.py:28-30,39,83` — its own
  `CONFIG_MARKER` / `INSTRUCTIONS_MARKER` / `CONTEXT_MARKER` literals and the
  `_INLINE_SITE` regex, plus `argv()`, which resolves the binary by substituting
  `PLUGIN_PREFIX` into an absolute path. After conversion `extract()` returns an
  empty corpus (the conformance suite parametrises to zero cases) and `argv[0]`
  becomes bare `accelerator`, not the installed `bin/accelerator` path the suite
  asserts. The bare form resolves via PATH, so the harness must locate the
  binary differently — not by textual substitution.

`tasks/shared/dispatch_coherence.py` consumes `LAUNCHER` / `BARE_LAUNCHER` /
`launcher_token` / `is_plugin_invocation` but needs **no source edit** — it
imports the constants, so the Change 1 redefinition carries it. Its behaviour
and tests must still be verified green (the over-broad-rule semantics shift when
`BARE_LAUNCHER` becomes bare).

**Tests that bake in the prefix (update red-first):**

- `tests/unit/tasks/test_skill_permissions.py` — `_RULE`, `_ACC`, the `!`
  fixtures, the bare-launcher grant fixture.
- `tests/unit/tasks/shared/test_skill_parsing.py:53-60,70-72` — the `covered_by`
  contract cases and the assertions that `covered_by(BARE_LAUNCHER, …)` matches
  the `${CLAUDE_PLUGIN_ROOT}/*` and `…bin/*` ancestor globs (both invert once the
  sentinel is bare).
- `tests/unit/tasks/shared/test_dispatch_coherence.py` — fixtures built from the
  `LAUNCHER` constant (self-adjusting) plus the over-broad case pairing a bare
  command with `${CLAUDE_PLUGIN_ROOT}/bin/*` rules, whose coverage relationship
  inverts.
- `tests/unit/tasks/test_integration_skills.py:19-84` — fixture skill bodies
  embedding `${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear …`, the input to the
  write-gate / keyword-parity guards.
- `tests/integration/skill-invocation/test_skill_invocation_conformance.py:104`
  — the `argv[0] == …/bin/accelerator` assertion over the corpus, which the
  `argv()` rework (Change 4) invalidates.

Out of scope: `tests/unit/tasks/test_e2e_docker.py` and
`tests/unit/tasks/shared/dev/test_circus.py` name the unrelated
`/bin/accelerator-visualiser` server binary; `tests/integration/hooks/*` are
hooks; `tasks/test/helpers.py:15` is a non-behavioural docstring.

The parser flip, the body conversion, and every consumer above are therefore
**coupled** — they must land in one change, tests-first.

The guarding lints are absent from the fast lane. `lint:skill-permissions:check`
and `lint:call-site-migration:check` appear only under `lint:check`
(`mise.toml:648`) and the full `default` (`mise.toml:668`); `mise run check`
(`mise.toml:662-664`) reaches neither, because `build-system:check`
(`mise.toml:506`) does not depend on them.

## Desired End State

Every `SKILL.md` invokes `accelerator …` bare across all three surfaces; the
three acceptance greps return zero; `is_plugin_invocation` and the permission
lint recognise the bare form and keep the 42-skill injection census intact; and
a new `lint:bare-invocation:check` fails — naming the offending file — on any
reintroduced prefix, absolute path, or `bash` wrapper, running in both the fast
`check` and `lint:check` lanes. `mise run check` and the full suite pass.

### Key Discoveries

- Three surfaces, not one — `skills/visualisation/visualise/SKILL.md:7` (grant)
  and `:13` (body) both carry the prefix; moving one without the other prompts
  the permission matcher.
- `is_plugin_invocation` gates the whole permissions lint on the prefix
  (`tasks/shared/skill_parsing.py:91-93`) — a silent-failure trap on conversion.
- The census equality `EXPECTED_INJECTION_SKILLS = 42`
  (`tasks/lint/skill_permissions.py:48`) turns a dropped-recognition bug into a
  loud failure — useful as a Phase 1 tripwire.
- `call_site_migration.py:26-82` is the exact scan template for the new lint:
  walk `skills/**/SKILL.md`, collect line hits, `Exit(code=1)`.
- Five non-accelerator `${CLAUDE_PLUGIN_ROOT}/skills/…` references exist (lens /
  output-format file paths, not accelerator invocations) — `is_plugin_invocation`
  must keep recognising the `${CLAUDE_PLUGIN_ROOT}/` prefix so their coverage is
  not dropped.

## What We're NOT Doing

- Not touching hooks — `hooks/hooks.json` dispatches via absolute placeholder
  paths through `bin/accelerator`; `bin` is not guaranteed on their PATH.
- Not changing the launcher bootstrap `bin/accelerator` or its self-location.
- Not moving `lint:skill-permissions:check` / `lint:call-site-migration:check`
  into the fast `check` lane — a pre-existing coverage gap, out of scope here.
- Not addressing the org-settings distribution caveat (a top-level `bin/` cannot
  ship through claude.ai organization settings) — epic 0136 distribution
  decision, tracked separately.
- Not merging or rewriting `call_site_migration.py`'s bash-config guard; the new
  lint is a separate module. Hosting the rule inside that guard (both are
  forbidden-substring scans over the same corpus) and extracting a shared
  skill-iteration helper — the `rglob("SKILL.md")` walk now recurs in five
  modules — were weighed and deferred as a follow-on tidy-up, to keep this
  change's diff to the convention flip.

## Implementation Approach

Three phases, each independently mergeable and green. The conversion and the
parser flip are one atomic phase because they are coupled; the new lint is a
second phase (it can only be added once the tree is clean); closing 0107 is a
third, doc-only phase.

### Precondition gate — PATH resolution for the `!` preprocessor

**Gating, manual, before Phase 1.** The Claude Code plugins reference states a
plugin's `bin/` is added to "the Bash tool's PATH" and invokable as a bare
command. The `!` preprocessor is a distinct execution surface the docs do not
name, and the pathed form being dropped is the older, universally-available
`${CLAUDE_PLUGIN_ROOT}` textual-substitution mechanism — so the gate must
establish not just that the bare form resolves, but that it resolves at the
supported floor and wins PATH precedence. Confirm empirically before bulk
conversion:

1. **Version floor.** Determine the Claude Code version that first adds the
   plugin `bin/` to the `!`-preprocessor and Bash-tool PATH (trace the
   changelog, as 0182 traced `${CLAUDE_PLUGIN_DATA}` to v2.1.78). Confirm it is
   at or below the declared minimum **v2.1.144**; if it is higher, either raise
   the plugin's minimum and record which installed versions lose support, or
   stop. Bin-on-PATH is a newer feature than textual substitution, so this is
   the primary risk, not a formality. Record the evidence **per surface**: the
   Bash-tool PATH addition is documented and changelog-traceable, but the `!`
   preprocessor is a surface the docs do not name, so a trace may find nothing
   for it. An inconclusive `!`-surface trace is the "version floor cannot be met
   → stop" branch — not a licence to proceed on the developer's newer installed
   version, which proves nothing about v2.1.144.
2. **Resolution + precedence.** Create a throwaway skill whose body is a single
   `` !`accelerator config path plans --fail-safe` `` (bare, no prefix). Invoke
   it in a main session and inside a subagent; confirm it resolves and emits the
   same output as the prefixed form. In the same run capture `command -v
   accelerator` and the resolved `$PATH` ordering, confirming the plugin `bin/`
   **precedes** typical user / system bin locations (the bare grant authorises
   whatever resolves first — see the security note in Migration Notes).
3. **Grant surface.** Confirm a model-issued Bash-**tool** `accelerator config …`
   call (not just a `!` command) both resolves on PATH and matches the bare
   `Bash(accelerator config *)` grant without prompting — the 97 grant
   conversions depend on the prefix-matcher accepting the bare token.
4. Delete the throwaway skill.

If the bare form does not resolve in either context, does not win precedence, or
the version floor cannot be met, **stop** — the item's central assumption is
false and the conversion is blocked. Record the outcome against work item
0245's Open Question (Phase 3).

## Phase 1: Reconcile the parser and permissions lint, then convert all skills

### Overview

Flip `skill_parsing.py` and `skill_permissions.py` to recognise the bare form,
then mechanically convert all 427 sites. One phase, because the lint recognition
and the body form must agree at every commit.

### Changes Required

#### 1. Shared parser recognises the bare launcher

**File**: `tasks/shared/skill_parsing.py`
**Changes**: Redefine the launcher constants to the bare form; widen
`is_plugin_invocation` to accept bare `accelerator` while still recognising the
`${CLAUDE_PLUGIN_ROOT}/` prefix for surviving non-accelerator plugin scripts.
`launcher_token` already keys off `LAUNCHER` with a required trailing space, so
it needs no change beyond the constant. `FORBIDDEN_LAUNCHER` names the outgoing
prefixed form in one place, so the Phase 2 lint scans for a named constant
rather than a fresh literal copy of the string the change eradicates.

```python
LAUNCHER = "accelerator"
BARE_LAUNCHER = f"{LAUNCHER} zz-external-subcommand-zz"
FORBIDDEN_LAUNCHER = f"{PLUGIN_PREFIX}bin/accelerator"


def is_plugin_invocation(command: str) -> bool:
    return (
        command.startswith(PLUGIN_PREFIX)
        or command == LAUNCHER
        or command.startswith(f"{LAUNCHER} ")
    )
```

`PLUGIN_PREFIX` stays defined — it still guards the non-accelerator
`${CLAUDE_PLUGIN_ROOT}/skills/…` references. Also expose the config-marker
strings here, derived from `LAUNCHER`, so the permissions lint and the
integration corpus consume one definition instead of each re-declaring
`accelerator config`:

```python
CONFIG_MARKER = f"{LAUNCHER} config "
CONTEXT_SKILL_MARKER = f"{LAUNCHER} config context --skill "
CONTEXT_ANY_MARKER = f"{LAUNCHER} config context"
INSTRUCTIONS_MARKER = f"{LAUNCHER} config instructions "
```

These markers encode the config-subcommand vocabulary, a second axis alongside
SKILL.md parsing; extend the module docstring so that shared launcher / config
vocabulary is stated as an intentional responsibility, not an implicit one.

#### 2. Permissions lint markers move to the bare form

**File**: `tasks/lint/skill_permissions.py`
**Changes**: Import the shared markers from `skill_parsing` instead of
re-declaring literals, so the launcher name is defined once.

```python
from tasks.shared.skill_parsing import (
    CONFIG_MARKER,
    CONTEXT_SKILL_MARKER,
    CONTEXT_ANY_MARKER,
    INSTRUCTIONS_MARKER,
)
```

Rename the four existing module-private references (`_CONFIG_MARKER`,
`_CONTEXT_SKILL`, `_CONTEXT_ANY`, `_INSTRUCTIONS`) throughout `_command_violations`
and `_check_skill` to the imported public identifiers, so one naming convention
holds. `tests/integration/support/skill_corpus.py` (Change 4) imports the same
markers rather than keeping its own copies. The markers stay substring tests
applied only to commands already accepted by `is_plugin_invocation`, so the
match still keys on a recognised launcher command; because `is_plugin_invocation`
now anchors the bare form to a leading `accelerator`/`accelerator ` (not a
free substring), a marker cannot match mid-argument.

#### 3. Write-gate mutation matcher recognises the bare form

**File**: `tasks/shared/skill_write_gate.py`
**Changes**: Retarget the mutation regex from the prefixed path to the bare
launcher.

```python
pattern = re.compile(rf"\baccelerator {provider} (?:{verbs})\b")
```

#### 4. Integration conformance corpus recognises the bare form

**File**: `tests/integration/support/skill_corpus.py`
**Changes**: Retarget the three markers and the `_INLINE_SITE` regex to the bare
form, and rework `argv()` so it resolves the binary the way the bare form does.
The prefixed form was resolved by substituting `PLUGIN_PREFIX` into an absolute
path (`argv()` at :83) and asserted as `argv[0] == …/bin/accelerator`
(`test_skill_invocation_conformance.py:104`). The minimal correct change: stop
substituting `PLUGIN_PREFIX` in `argv()` and drop that absolute-path `argv[0]`
assertion, leaving `run_bootstrap`'s `installation.root` resolution intact (it
already locates the binary and passes only `argv[1:]`). Do **not** inject
`ACCELERATOR_BIN` — that would reintroduce the exact empty-environment gap this
suite exists to guard (no plugin root, no injected binary, the production `!`
shape).

Import the three config markers from `skill_parsing` (Change 1) rather than
re-declaring them; only `_INLINE_SITE` remains local:

```python
from tasks.shared.skill_parsing import (
    CONFIG_MARKER,
    CONTEXT_SKILL_MARKER as CONTEXT_MARKER,
    INSTRUCTIONS_MARKER,
)

_INLINE_SITE = re.compile(r"!`[^`]*\baccelerator config ")
```

#### 5. Mechanical conversion of every call site

**File**: `skills/**/SKILL.md` (47 files)
**Changes**: Replace `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` with `accelerator`
across bodies, grants, prose, and fenced examples.

```diff
-  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
+  - Bash(accelerator config *)
-!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill visualise --fail-safe`
+!`accelerator config context --skill visualise --fail-safe`
```

### TDD Notes

Write the failing tests first across every coupled test file, then apply the
source changes to green:

- `tests/unit/tasks/test_skill_permissions.py` — update `_RULE`, `_ACC`, the `!`
  fixtures, and the bare-launcher grant fixture to the bare form; assert the
  census still resolves to 42.
- `tests/unit/tasks/shared/test_skill_parsing.py` — update the `covered_by`
  contract cases (including `:43-44`, where `(_VISUALISE, PLUGIN_PREFIX+'*',
  True)` and `…bin/*` invert to `False` for a bare command) and the
  `BARE_LAUNCHER` ancestor-glob assertions (`:70-72`) for the bare sentinel; add
  a positive case proving a bare `accelerator …` command is recognised by
  `is_plugin_invocation`, and a negative-boundary case proving a sibling binary
  (`accelerator-verify-darwin-arm64 x`) is **not**. Also convert a
  `test_launcher_token` regression case (`:94-96`) to the bare form — e.g.
  `("accelerator-verify-darwin-arm64 x", "")` — so the trailing-separator guard
  stays covered; under bare `LAUNCHER` the prefixed sibling inputs return early
  and no longer exercise it.
- `tests/unit/tasks/shared/test_dispatch_coherence.py` — re-derive
  `TestOverBroadSkills` rather than relabel it: keep at least one genuine
  bare-form ancestor-glob case (`accelerator *`) asserting the over-broad veto
  fires, and re-purpose the two `${CLAUDE_PLUGIN_ROOT}/…` params (which collapse
  to plain non-coverage of a bare command) to assert they simply fail to bind,
  so both behaviours stay pinned. Keep `test_the_real_skills_tree_passes` green
  on the converted tree.
- `tests/integration/skill-invocation/test_skill_invocation_conformance.py` —
  the hard `argv[0] == …/bin/accelerator` assertion (`:104`) fails once `argv()`
  yields `['accelerator', …]`. **Replace** it (assert the bare `accelerator`
  token, or the resolved bootstrap path `run_bootstrap` derives from
  `installation.root`) rather than deleting it, so the corpus-to-bootstrap
  wiring stays pinned; `_ALL = corpus.extract(_SKILLS)` and `TestCorpusIntegrity`
  then run against the bare-form corpus.
- `tests/unit/tasks/test_integration_skills.py` — convert the
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear …` fixture bodies to the bare
  form.

Run red, apply changes 1-4 green, then perform change 5 (the bulk conversion),
and confirm the integration conformance suite passes against the converted
tree.

### Success Criteria

#### Automated Verification

- [x] Coupled unit tests pass: `uv run pytest tests/unit/tasks/test_skill_permissions.py tests/unit/tasks/shared/test_skill_parsing.py tests/unit/tasks/shared/test_dispatch_coherence.py tests/unit/tasks/test_integration_skills.py`
- [x] Integration conformance suite passes on the converted tree: `mise run test:integration:skill-invocation`
- [x] Permissions lint green: `mise run lint:skill-permissions:check`
- [x] Write-gate / keyword-parity lint green: `mise run lint:integration-skills:check`
- [x] Dispatch guard green: `mise run lint:dispatch-coherence:check`
- [x] No prefix remains: `grep -rF '${CLAUDE_PLUGIN_ROOT}/bin/accelerator' skills` returns zero
- [x] No path suffix remains: `grep -rF '/bin/accelerator' skills` returns zero
- [x] No shell wrapper: `grep -rEn '(^|[^[:alnum:]_])(bash|sh|env) [^`]*accelerator' skills` returns zero (leading-boundary anchor avoids matching `sh` inside `publish`)
- [x] Fast lane green (runs lint but not tests): `mise run check`
- [x] Build-system component green: `mise run build-system:check`

#### Manual Verification

- [x] Precondition gate passed in both main session and subagent (see gate section)
- [ ] Spot-check a converted skill (`/accelerator:visualise`) loads and runs with no permission prompt

---

## Phase 2: Add the bare-invocation allowlist lint and wire it into check

### Overview

A new lint module owning 0245's rule, guarding all three forbidden forms and
running in both the fast `check` and `lint:check` lanes.

### Changes Required

#### 1. New lint module

**File**: `tasks/lint/bare_invocation.py`
**Changes**: A pure `violations(root)` helper plus a thin `@task check`,
matching the entry-point shape of the sibling guards (`call_site_migration.py`,
`skill_permissions.py`) rather than a divergent `scan_*` name. Scan
`skills/**/SKILL.md` line-by-line for three forbidden forms and `Exit(code=1)`
listing `file:line` per hit:

- the `/bin/accelerator` path suffix (available as `FORBIDDEN_LAUNCHER` minus
  the prefix, or scanned directly) — catches both the
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` prefix and any absolute-path render in
  one substring;
- a `bash`/`sh`/`env` wrapper at the command position — anchor the wrapper
  token to the start of the invocation (`^\s*(?:bash|sh|env)\b`, applied after
  stripping the `` !` `` or fence prefix), so the guard covers the full wrapper
  set work item 0107 required, not just `bash`. A raw substring
  `(bash|sh|env)\s+…accelerator` over whole lines over-matches — `sh` occurs
  inside `publish`, so prose like "publish accelerator directly" would flag —
  hence the leading-boundary anchor, not a free substring.

```python
def violations(root: Path) -> list[str]:
    ...
```

Each entry is one `<rel>:<line>: <form>` string, one per hit across
`skills/**/SKILL.md`.

The scan must not flag the five legitimate `${CLAUDE_PLUGIN_ROOT}/skills/…`
lens / output-format references — they contain `/skills/`, not
`/bin/accelerator`, so keying on `/bin/accelerator` spares them by construction.
The `(bash|sh|env)…accelerator` regex is line-greedy and will also flag prose
that legitimately names the launcher (the `visualise` skill already carries
launcher prose); anchor it to the line's leading command token, or reserve a
documented opt-out, so documentation is not trapped.

#### 2. Register the task (six-step wiring)

- **File**: `tasks/lint/__init__.py` — add `bare_invocation` to the import and
  `__all__`.
- **File**: `tasks/__init__.py` — `ns_lint.add_collection(Collection.from_module(lint.bare_invocation))`.
- **File**: `mise.toml` — add the façade and register it in two roll-ups.

```toml
[tasks."lint:bare-invocation:check"]
description = "Guard the bare accelerator invocation convention across skills/**/SKILL.md (no ${CLAUDE_PLUGIN_ROOT}/bin prefix, absolute path, or bash wrapper)"
depends = ["deps:install:python"]
run = "invoke lint.bare-invocation.check"
```

Add `"lint:bare-invocation:check"` to `build-system:check` (`mise.toml:506`, so
the fast `check` covers it) and to `lint:check` (`mise.toml:648`).

- **File**: `tests/unit/tasks/test_mise.py` — add `"lint:bare-invocation:check"`
  to `_BUILD_SYSTEM_CHECK_GATES`. This is a skills-tree guard, exactly like
  `lint:dispatch-coherence:check` and `lint:integration-skills:check`, so it
  belongs in that list; the parametrised `test_gate_wired_into_build_system_check`
  and `test_gate_wired_into_lint_check` then pin its placement against silent
  unwiring. Without this step the new gate is the only skills-tree guard whose
  placement nothing pins.

### TDD Notes

Write `tests/unit/tasks/test_bare_invocation.py` first (red), covering every
forbidden form and both carve-outs:

- a fixture skill carrying the `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` prefix
  yields a violation naming the file;
- a fixture skill carrying an absolute-path render yields a violation;
- fixture skills wrapped as `bash`, `sh`, and `env` each yield a violation (the
  `sh`/`env` cases guard against regressing to the `bash`-only subset);
- a clean bare-form skill yields none;
- a skill carrying a legitimate `${CLAUDE_PLUGIN_ROOT}/skills/…` lens /
  output-format reference yields none (pins the deliberate carve-out against a
  future over-tightened pattern).

Then implement the module green.

### Success Criteria

#### Automated Verification

- [x] New unit tests pass: `uv run pytest tests/unit/tasks/test_bare_invocation.py`
- [x] Gate-wiring pinned: `uv run pytest tests/unit/tasks/test_mise.py` (the new gate is in `_BUILD_SYSTEM_CHECK_GATES`)
- [x] Lint green on the converted tree: `mise run lint:bare-invocation:check`
- [x] Covered by the fast lane: `mise run check` runs the new lint and passes
- [x] Reintroduction fails: adding a prefixed call to a skill makes the lint exit non-zero and name that file
- [x] `sh`/`env` wrappers also fail: a scratch skill wrapping the call in `sh`/`env` makes the lint exit non-zero

#### Manual Verification

- [x] The failure message reads clearly and points to the exact `file:line`

---

## Phase 3: Close 0107 and record the linkage

### Overview

0245 owns the bare-invocation lint, superseding 0107. Reconcile the work items;
doc-only, within `meta/`.

### Changes Required

#### 1. Supersede 0107

**File**: `meta/work/0107-lint-skill-body-script-invocations.md`
**Changes**: Transition status to `done` (or `abandoned`) — `superseded` is
**not** a valid work-item status (the vocabulary is `draft, ready, in-progress,
review, done, blocked, abandoned`; `superseded` exists only for `plan` / `adr`
/ `design-inventory`), so setting it would fail this phase's own frontmatter
validation. Record the supersession in prose and via the `relates_to` linkage
list (there is no `supersedes` / `superseded_by` key for work items, unlike
ADRs), noting the built rule lives in `tasks/lint/bare_invocation.py`.

#### 2. Record ownership and the closed Open Question on 0245

**File**: `meta/work/0245-invoke-accelerator-directly-in-skills.md`
**Changes**: Note the lint-ownership in Dependencies; record the precondition
gate's affirmative outcome (including the established version floor and the PATH
precedence result), closing the PATH Open Question. Record bin-on-PATH as a
tracked external Claude Code dependency alongside 0182's documented behaviours,
and state the revert path: because the bare form is unconditional and the lint
forbids the pathed form, a future Claude Code regression that stops adding
`bin/` to PATH breaks all 42+ injection skills at load, and reverting requires a
coordinated lint change — this is the accepted trade-off (no per-call-site
fallback), not an oversight.

#### 3. Record the intentional hooks boundary durably

**File**: a new `hooks/CLAUDE.md` (matching the per-directory-gotcha convention
of `tasks/CLAUDE.md` and `cli/visualiser/CLAUDE.md`). Not `hooks.json` — it is
strict JSON and cannot carry a comment.
**Changes**: Note that hooks intentionally retain the
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator` pathed form and must **not** be
converted to the bare form — hooks receive `${CLAUDE_PLUGIN_ROOT}` as a real
exported env var but may lack `bin` on their PATH. The new lint only scans
`skills/**/SKILL.md`, so nothing else prevents a contributor from "unifying"
hooks onto the bare form and breaking them; the rationale must not live only in
this plan.

### Success Criteria

#### Automated Verification

- [ ] Frontmatter validates: `accelerator corpus frontmatter validate --file meta/work/0107-lint-skill-body-script-invocations.md`
- [ ] Frontmatter validates: `accelerator corpus frontmatter validate --file meta/work/0245-invoke-accelerator-directly-in-skills.md`

#### Manual Verification

- [ ] 0107 no longer implies an unbuilt lint; 0245 records the resolved Open Question

---

## Testing Strategy

### Unit Tests

- Bare-form recognition in `is_plugin_invocation` and the permissions census
  (Phase 1).
- Each forbidden form flagged, clean form passed, offender named (Phase 2).

### Integration Tests

- `mise run check` green end-to-end (fast lane now includes the new lint).
- `mise run lint:check` green (skill-permissions on the bare tree, new lint).

### Manual Testing Steps

1. Load `/accelerator:visualise` and one config-reading skill; confirm no
   permission prompt and identical output.
2. Reintroduce a prefixed call in a scratch skill; confirm `mise run check`
   fails naming the file; revert.

## Migration Notes

No data migration. The conversion is behaviour-preserving **conditional on the
verified PATH precondition** — the two forms do not have identical resolution
dependencies. The prefix form resolves by textual `${CLAUDE_PLUGIN_ROOT}`
substitution into skill content (older, guaranteed, PATH-independent); the bare
form resolves by the plugin `bin/` being on the `!`-shell and Bash-tool PATH.
There exists an environment where the prefix form works but the bare form does
not: any Claude Code that substitutes `${CLAUDE_PLUGIN_ROOT}` textually but does
not export the plugin `bin/` onto that PATH. The precondition gate exists to
rule that out; the equivalence is not unconditional.

The gate's precedence check (step 2) observes one environment at one time. PATH
ordering is environment- and OS-dependent, so an end-user whose PATH already
holds an unrelated `accelerator` earlier, or whose plugin `bin/` is ordered
later, would resolve the bare call to the wrong binary with no per-call-site
fallback. This is a real guarantee only if Claude Code *prepends* the plugin
`bin/` (established from the step-1 changelog trace, not the local check). Where
that cannot be established, consider having the launcher assert its own identity
or the conformance corpus pin the resolved path, so a name collision surfaces
loudly rather than silently — tracked with the bin-on-PATH dependency in Phase
3.

**Security tradeoff (accepted, recorded).** The prefix grant
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)` pins the shipped plugin
binary regardless of PATH; the bare grant `Bash(accelerator config *)`
authorises whatever `accelerator` resolves first on PATH, run unconditionally at
skill load. Under a PATH-shadowing compromise (an actor who can reorder PATH or
drop a binary — substantial pre-existing access) the bare grant pre-authorises
running that binary with no prompt, and the new lint forecloses re-pinning to
the explicit path as defence-in-depth. The residual risk is low for a
developer-tooling surface; it is accepted in exchange for one uniform call form.
The real assurance the plugin binary wins is a fixed *prepend* of the plugin
`bin/` (established from the step-1 changelog trace); the step-2 check is a
one-time local observation, not a per-environment guarantee.

## References

- Original work item: `meta/work/0245-invoke-accelerator-directly-in-skills.md`
- Related research: `meta/research/codebase/2026-09-06-0245-invoke-accelerator-directly-in-skills.md`
- Superseded item: `meta/work/0107-lint-skill-body-script-invocations.md`
- Scan template: `tasks/lint/call_site_migration.py:26-82`
- Coupled surfaces: `skills/visualisation/visualise/SKILL.md:7,13`
- Recognition trap: `tasks/shared/skill_parsing.py:91-93`
- Write-gate matcher: `tasks/shared/skill_write_gate.py:32`
- Conformance corpus: `tests/integration/support/skill_corpus.py:28-30,39,83`
- Census tripwire: `tasks/lint/skill_permissions.py:48`
- Gate-placement pins: `tests/unit/tasks/test_mise.py` (`_BUILD_SYSTEM_CHECK_GATES`)
- Lane wiring: `mise.toml:506,648,664`
