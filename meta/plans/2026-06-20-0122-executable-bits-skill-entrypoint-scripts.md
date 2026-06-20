---
type: plan
id: "2026-06-20-0122-executable-bits-skill-entrypoint-scripts"
title: "Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts Implementation Plan"
date: "2026-06-20T17:07:39+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0122"
parent: "work-item:0122"
derived_from: ["codebase-research:2026-06-20-0122-executable-bits-skill-entrypoint-scripts"]
relates_to: ["work-item:0106", "work-item:0098", "work-item:0107"]
tags: [scripts, permissions, ci, lint, plugin, executable-bit]
revision: "b717ec4ab24f1982ec33310679897f11a21db0cd"
repository: "build-system"
last_updated: "2026-06-20T18:46:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts Implementation Plan

## Overview

Tracked `.sh` scripts that are meant to run directly as executables (invoked by
bare path from skill bodies, hooks, migrations, and test runners under the 0106
bare-path convention) are inconsistently marked: some entrypoints lack the
executable bit, and some sourced-only libraries wrongly carry it. This plan
establishes a single invariant — **a tracked `.sh` is executable (`100755`) if
and only if it is _not_ on a checked-in library-list** — by (1) correcting the
~23 mode mismatches found in the live audit, (2) adding a bidirectional,
pure-Python CI guard in the `lint:shell` family that enforces the invariant via
`shell_sources()`, and (3) documenting the mechanism so the condition cannot
recur.

## Current State Analysis

The exec bit is a tracked VCS attribute (`100755` vs `100644`), and under 0106 it
is **load-bearing**: a bare-path-invoked entrypoint without `+x` fails with
"permission denied" or escapes its `allowed-tools` permission. A live `stat`
sweep of the working tree (excluding `node_modules/`, `workspaces/`, `target/`),
cross-referenced with call-site classification, was performed by the research and
**re-confirmed mode-by-mode during this planning session** (`stat -f '%Lp'`). It
yields a correction set of **23 mode changes**, materially larger than the 6 the
work item names:

- **18 entrypoints at `644` that must become `755`** — the two named
  (`migrations/0004-…sh`, `test-interactive-protocol.sh`) plus 6 jira
  `*-flow.sh` and 10 `work-item-*.sh` scripts that are invoked identically to
  their already-`755` jira/linear/work-item siblings.
- **5 libraries at `755` that must become `644`** — the four named
  (`atomic-common.sh`, `config-common.sh`, `vcs-common.sh`,
  `work-item-common.sh`) plus a fifth the research found:
  `skills/visualisation/visualise/scripts/test-helpers.sh` (sourced-only yet
  `755`, contradicting `tasks/test/helpers.py:20-22`).

The shell-lint toolchain from 0098 is the guard's natural home:
`tasks/lint/scripts.py` (`shellcheck` + `bashisms` `@task`s) follows a fixed
idiom — enumerate via `shell_sources()`, **fail-closed on empty scope**
(`_EMPTY_SCOPE`), raise a single `invoke.Exit(message, code=1)` listing
offenders. The guard auto-registers (`Collection.from_module(lint.scripts)` in
`tasks/__init__.py:71`) with no `__init__.py` edit; it is wired into CI by adding
a `mise.toml` task block and one `depends` entry.

### Key Discoveries:

- **`shell_sources()` (`tasks/shared/sources.py:60-100`)** is the mandated,
  jj-safe enumeration: an `os.walk` honouring the root `.gitignore`, pruning
  `workspaces/` via `_keep` (`:29-37`), appending the extensionless
  `accelerator-visualiser` via `_EXTRA_SHELL_SOURCES` (`:55-57`). It already
  excludes `node_modules/`/`target/` for free (AC5).
- **`shell_sources()` keeps `test-fixtures/**`** — a deliberate 0098 widening
  asserted by `tests/unit/tasks/shared/test_sources.py:28-45`. Eight `644`
  fixture scripts under `skills/config/migrate/scripts/test-fixtures/**` are run
  via `bash "$f"` (the migration runner globs by `find -name`, never by exec
  bit — `run-migrations.sh:163,273`) so they never need `+x`. As specified, the
  bidirectional guard would wrongly demand `+x` on all eight. **Decision: the
  guard exempts any path with a `test-fixtures/` segment** (see Decisions).
- **Dual-use scripts validate AC1's wording.** `linkage-parser.sh`,
  `validate-source.sh`, and `jira-fields.sh` are each `source`d (by tests) _and_
  invoked by path (in production); all three are correctly `755`. "Sourced ⇒
  library" alone would wrongly strip their `+x` — only AC1's "≥1 source ref
  **and zero** path invocations" classifies them correctly as entrypoints.
- **`accelerator-visualiser`** (extensionless, `_EXTRA_SHELL_SOURCES`) is an
  entrypoint, already `755` (confirmed). The guard treats it as off-list → must
  be `755`; no change needed, but it must not be miscounted.
- **Guard reads the working-copy mode** (`os.access(p, os.X_OK)`, as
  `tasks/test/helpers.py:38`), while the ACs speak of committed VCS mode. These
  agree in practice because the bit is tracked and a committed `chmod`
  propagates; the only gap is a local uncommitted `chmod` (worth a one-line
  doc/test note, not a behavioural difference).
- **Test templates exist**: `tests/unit/tasks/test_lint.py` (mocked-`Context` +
  behavioural `tmp_path` layers), `tests/unit/tasks/test_integration.py:27-43`
  (`chmod` exec-bit precedent), `tests/unit/tasks/shared/test_sources.py`
  (`root=tmp_path` seam), `tests/unit/tasks/test_python_coverage.py:133-176`
  (anti-vacuous-pass sentinel probe).

### Decisions (resolving the research's Open Questions)

1. **Fixtures → guard-level `test-fixtures/` path exemption.** The guard skips
   any enumerated path containing a `test-fixtures/` segment, in both directions
   (neither required executable nor required on the list). `shell_sources()` is
   left untouched (preserving 0098's shellcheck/shfmt coverage of fixtures) and
   the library-list stays semantically clean (fixtures are bash-run fixtures,
   not libraries).
2. **Library-list → a Python constant.** The manifest is a module-level
   `frozenset[str]` of repo-relative POSIX paths in `tasks/lint/scripts.py`,
   beside the guard. It is type-checked, directly testable (ruff `SLF001`
   relaxed in tests), and the documentation in `tasks/README.md` points
   contributors to it.
3. **Full ~23-change scope; escalation recorded as a note.** The plan corrects
   all 23 mismatches and registers the complete library-list. The work item's
   high-priority escalation trigger _has fired_ — the 6 jira flows and the
   work-item sync scripts are packaged, bare-path-invoked entrypoints missing
   `+x` (the strict 0106 case) — so this is captured as a note/follow-up for the
   release/packaging owner (Migration Notes), without blocking the plan.

## Desired End State

On the committed tree: every tracked `.sh` not on the library-list (and not under
`test-fixtures/`) has mode `100755`; every library-list member has mode
`100644`. Note this codifies a *convention* — uniform `755` for every
shebang-bearing entrypoint — rather than a per-script functional requirement:
only the Python suite runner (`tasks/test/helpers.py:38`) and bare-path SKILL.md
invocation strictly need the bit, while sibling/migration `bash X` calls never
stat it. The uniform rule is the right consistency choice, but a future
maintainer should understand it as a convention that can be revisited if 0106's
mechanics change. A bidirectional guard in `mise run check` fails — naming each offender
— when an off-list entrypoint lacks `+x`, when a library-list member carries
`+x`, or when a library-list path no longer exists; and passes when the tree and
list are both correct. The mechanism is documented in `tasks/README.md`.

Verification: `mise run check` exits 0 on the corrected tree; flipping any single
bit (or adding a stale list entry) makes it exit non-zero naming the file;
`reinstall_chrome_stable_linux.sh` under `node_modules/` never enters the guard's
input set.

## What We're NOT Doing

- **No `mise run fix` autofixer.** The guard reports only; correction stays a
  manual `chmod`, preserving the established "shell has no autofixer" convention
  (`scripts` is absent from `lint:fix`). This is a settled work-item decision.
- **No change to `shell_sources()`** — the fixture handling is a guard-local
  exemption, not an enumeration change.
- **No re-classification of dual-use scripts** — `linkage-parser.sh`,
  `validate-source.sh`, `jira-fields.sh` are entrypoints (already `755`), stay
  off the list.
- **No 0107 work** (skill-body invocation lint). It shares the
  `tasks/lint/scripts.py` extension point but is out of scope; 0107 rebases
  against this plan's wiring.
- **No widening of the bashisms/shellcheck rule sets.**

## Implementation Approach

Three phases, each a complete, green, independently-mergeable PR. Phases 1 and 2
are **order-independent** of each other; Phase 3 is the integration step that
depends on both.

- **Phase 1 — Correct the modes** (pure `chmod`, no guard). Mergeable on its own:
  it fixes the latent bugs and leaves `mise run check` green. Independent of
  everything.
- **Phase 2 — Library-list + guard + tests + docs, _not_ wired into CI.**
  Mergeable on its own and independent of Phase 1: the guard's tests run against
  synthetic `tmp_path` trees, so they pass regardless of the real tree's modes,
  and nothing yet runs the guard over the real repo.
- **Phase 3 — Wire the guard into `mise run check`.** This is the only step that
  runs the guard over the real tree, so it must land after both Phase 1 (modes
  corrected) and Phase 2 (guard exists). It adds the real-tree integration
  assertion.

TDD applies squarely to Phase 2 (write guard tests red, implement to green) and
to Phase 3's integration assertion. Phase 1 is mechanical `chmod`s not amenable
to TDD; its regression net is the guard delivered in Phases 2–3.

---

## Phase 1: Correct the Executable Bits

### Overview

Apply the 23 mode corrections so the *executable ⟺ off-the-library-list*
invariant holds on the committed tree. Mechanical and self-contained; no guard
yet.

### Changes Required:

#### 1. Set `+x` on 18 entrypoints (`644 → 755`)

`chmod +x` (committed, so the VCS records `100755`):

```
skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh
scripts/test-interactive-protocol.sh
skills/integrations/jira/scripts/jira-attach-flow.sh
skills/integrations/jira/scripts/jira-comment-flow.sh
skills/integrations/jira/scripts/jira-create-flow.sh
skills/integrations/jira/scripts/jira-search-flow.sh
skills/integrations/jira/scripts/jira-transition-flow.sh
skills/integrations/jira/scripts/jira-update-flow.sh
skills/work/scripts/work-item-fetch-remote.sh
skills/work/scripts/work-item-file-dirty.sh
skills/work/scripts/work-item-normalise.sh
skills/work/scripts/work-item-project-remote.sh
skills/work/scripts/work-item-section-diff.sh
skills/work/scripts/work-item-sync-apply.sh
skills/work/scripts/work-item-sync-baseline.sh
skills/work/scripts/work-item-sync-classify.sh
skills/work/scripts/work-item-sync-decide.sh
skills/work/scripts/work-item-update-remote.sh
```

#### 2. Clear `+x` on 5 libraries (`755 → 644`)

`chmod -x` (committed, so the VCS records `100644`):

```
scripts/atomic-common.sh
scripts/config-common.sh
scripts/vcs-common.sh
skills/work/scripts/work-item-common.sh
skills/visualisation/visualise/scripts/test-helpers.sh
```

#### 3. Re-run the audit before freezing

Before committing, re-run the `stat -f '%Lp'` sweep over the full
`shell_sources()` set (and `accelerator-visualiser`) to confirm exactly these 23
files changed and every other tracked `.sh` already satisfies the invariant
against the Phase-2 library-list. Crucially, **stat every member of the
(reconciled) library-list, not just the four named suspects** — any list member
found at `755` must be added to the `chmod -x` set above rather than assumed
already `644`. The seed libraries claimed correct-as-is
(`accelerator-scaffold.sh` → `644`, `doc-type-inference.sh` → `644`, etc.) are
only correct if the live stat confirms it; do not rely on the planning-session
premise. Confirm the four named suspects land on their expected outcomes
(`migrations/0004-…sh` → `755`, `test-interactive-protocol.sh` → `755`,
`accelerator-scaffold.sh` → `644`, `doc-type-inference.sh` → `644`).

### Success Criteria:

#### Automated Verification:

- [x] `mise run check` exits 0 (no regressions introduced by the mode changes):
      `mise run check`
- [x] All shell test suites still discovered and pass (the exec-bit discovery
      count is non-zero — `run_shell_suites` fails loudly on a dropped bit):
      `mise run test` — the newly-executable `test-interactive-protocol.sh` is
      discovered and passes (24/0/0). Two unrelated, pre-existing failures remain
      in `mise run test` (NOT caused by the mode changes, which are exec-bit-only
      and cannot affect them): a corpus-frontmatter violation
      (`meta/reviews/work/0118-reconcile-0007-backfill-sentinel-with-validator-review-1.md`
      carries an empty `relates_to: []` — same class as commit `d7f1df88f`), and
      the `test:e2e:visualiser` resolved-styles specs. Both predate this work and
      are out of scope.
- [x] A `stat` sweep confirms each of the 18 entrypoints is `755` and each of the
      5 libraries is `644` (working-copy modes match committed modes).

#### Manual Verification:

- [x] `jj diff --summary` (run from within the build-system workspace) shows the
      23 files as mode-only changes (no content edits), and the recorded modes
      are the committed `100755`/`100644` — not merely a working-copy `chmod`
      (this is the property a fresh CI checkout sees, which the guard's
      working-copy read approximates).
- [x] Spot-invoke a corrected entrypoint by bare path (e.g.
      `skills/work/scripts/work-item-sync-classify.sh`) and confirm it executes
      rather than "permission denied".

---

## Phase 2: Library-List Constant, Bidirectional Guard, Tests, and Docs

### Overview

Add the checked-in library-list (Python constant), the pure-Python bidirectional
guard `@task`, its TDD tests, and the `tasks/README.md` documentation. **Not yet
wired into `mise run check`** — so this phase is green and independently
mergeable regardless of Phase 1's mode state (its tests use synthetic trees).

### Changes Required:

#### 1. The library-list constant

**File**: `tasks/lint/scripts.py`
**Changes**: Add a module-level `frozenset[str]` of the sourced-only,
repo-relative POSIX paths confirmed by the audit. Membership is defined **by
rule, not by count**: a path belongs iff it has ≥1 `source`/`.` reference **and
zero** bare-path or `bash`/`sh`/`env`-prefixed invocations. Do not anchor any
prose or code comment to a cardinality — the authoritative set is whatever the
AC1 re-derivation below produces, and the integrity test asserts exact
membership so the literal cannot silently drift.

```python
# Sourced-only shell libraries: loaded via `source`/`.`, never invoked by path.
# The guard enforces *executable iff NOT on this list*, so a tracked .sh absent
# here is treated as an entrypoint and must be 0755. A NEW sourced-only library
# MUST be added here or the guard will demand +x on it. See the
# "Executable-bit invariant" subsection in tasks/README.md.
SHELL_LIBRARIES: frozenset[str] = frozenset({
    "scripts/fs-common.sh",
    "scripts/hash-common.sh",
    "scripts/jsonl-common.sh",
    "scripts/log-common.sh",
    "scripts/work-common.sh",
    "scripts/config-defaults.sh",
    "scripts/config-common.sh",
    "scripts/atomic-common.sh",
    "scripts/vcs-common.sh",
    "scripts/doc-type-table.sh",
    "scripts/doc-type-inference.sh",
    "scripts/frontmatter-emission-rules.sh",
    "scripts/frontmatter-fixtures.sh",
    "scripts/interactive-harness.sh",
    "scripts/interactive-protocol.sh",
    "scripts/test-helpers.sh",
    "scripts/accelerator-scaffold.sh",
    "skills/config/migrate/scripts/interactive-lib.sh",
    "skills/github/scripts/test-helpers.sh",
    "skills/visualisation/visualise/scripts/launcher-helpers.sh",
    "skills/visualisation/visualise/scripts/test-helpers.sh",
    "skills/work/scripts/work-item-common.sh",
    "skills/work/scripts/work-item-bridge-codes.sh",
    "skills/integrations/jira/scripts/jira-common.sh",
    "skills/integrations/jira/scripts/jira-auth.sh",
    "skills/integrations/jira/scripts/jira-jql.sh",
    "skills/integrations/jira/scripts/jira-body-input.sh",
    "skills/integrations/jira/scripts/jira-custom-fields.sh",
    "skills/integrations/linear/scripts/linear-common.sh",
    "skills/integrations/linear/scripts/linear-auth.sh",
})
```

> **Blocking gate (AC1 re-derivation).** The literal above is a *starting point
> from the planning-session audit, not the authoritative set*. Before this phase
> may be marked complete, re-derive the exact membership against the tree: for
> each candidate path, a repo-wide search over the `shell_sources()` corpus plus
> the skill/agent/hook surface (`SKILL.md` bodies, `agents/`, `hooks/`,
> mise/invoke tasks) must find ≥1 `source`/`.` reference **and zero** bare-path
> or `bash`/`sh`/`env`-prefixed invocations; add or remove entries as the sweep
> dictates. Pay particular attention to the jira/linear directories, which
> contain several helper-looking scripts (`jira-request.sh`, `jira-adf-to-md.sh`,
> `linear-graphql.sh`, …) that were *not* individually classified during planning
> — each must be resolved to entrypoint (off-list) or library (on-list) by the
> two-part rule. Record the reconciled set; the integrity test (§3) then pins it
> via exact set-equality so a later transcription slip turns the build red.

#### 2. The bidirectional guard `@task`

**File**: `tasks/lint/scripts.py`
**Changes**: Add a pure-Python `@task` (no external scanner) mirroring the
existing fail-closed idiom. Logic per enumerated path from `shell_sources()`:

```python
# Bash-run migration fixtures: discovered by name and executed via `bash "$f"`
# (never by exec bit, never sourced), so they are neither entrypoints nor
# libraries. Exempt from the invariant in both directions. The exemption is a
# path-segment match because shell_sources() returns POSIX-relative paths
# (see tasks/shared/sources.py); a future second fixture root would need adding
# here. A test asserts this segment matches only the known fixture tree.
_FIXTURE_SEGMENT = "test-fixtures"


@task
def exec_bits(context: Context) -> None:
    """Enforce: a tracked .sh is executable iff NOT on SHELL_LIBRARIES."""
    sources = shell_sources()
    if not sources:
        raise Exit(f"exec-bits: {_EMPTY_SCOPE}", code=1)

    repo = repo_root()
    in_scope = set(sources)
    offenders: list[str] = []

    # Stale-entry guard: every library-list path must still be enumerated by
    # shell_sources(). Keying on `in_scope` (not mere on-disk existence) closes
    # the gap where a library that exists but has left scope — gitignored,
    # relocated under workspaces/, or lost its .sh extension — would otherwise
    # pass the existence check yet never be mode-checked below.
    for rel in sorted(SHELL_LIBRARIES):
        if rel not in in_scope:
            offenders.append(
                f"stale library-list entry (not enumerated): {rel}  "
                "-> remove from SHELL_LIBRARIES or restore the file"
            )

    for rel in sources:
        if _FIXTURE_SEGMENT in rel.split("/"):
            continue
        executable = os.access(repo / rel, os.X_OK)
        # Each line is a runnable chmod; the "then commit" reminder is in the
        # per-offender comment (not only the preamble) because the working-copy
        # bit alone does not satisfy CI — see the Working-copy-mode stance.
        # Keep the command itself paste-safe (no fake `&& commit` that errors).
        if rel in SHELL_LIBRARIES and executable:
            offenders.append(f"chmod -x {rel}  # library -> 0644, then commit")
        elif rel not in SHELL_LIBRARIES and not executable:
            offenders.append(f"chmod +x {rel}  # entrypoint -> 0755, then commit")

    if offenders:
        raise Exit(
            "exec-bit invariant violated (a tracked .sh is executable iff it "
            "is NOT a sourced-only library). Run each line below AND COMMIT the "
            "mode change (shell has no autofixer; the bit must be committed to "
            "satisfy CI). If you believe a file is mis-classified, see the "
            '"Executable-bit invariant" subsection in tasks/README.md:\n  '
            + "\n  ".join(offenders),
            code=1,
        )
```

Add `import os` in alphabetical stdlib order (before the existing `import
shlex`) so the import block stays ruff `I001`-clean. Keep the new constant,
comment, and offender f-strings within the 80-column floor.

**Working-copy-mode stance (deliberate).** `os.access(p, os.X_OK)` reads the
*working-copy* mode, while the invariant is ultimately about the committed VCS
mode a fresh clone sees. This is an intentional, documented choice: it matches
the existing `tasks/test/helpers.py:38` precedent and keeps the guard a plain
filesystem walk. The two agree in practice because the exec bit is tracked and a
committed `chmod` propagates. The known gaps are (i) a *local uncommitted*
`chmod` (caught at commit time and by the Phase 1 `jj diff` verification, which
asserts the modes are actually recorded as `100755`/`100644`), and (ii) an
exec-bit-lossy or bit-synthesising filesystem — out of scope given the macOS +
Linux target matrix (CI runs `check-scripts` on `ubuntu-latest`; local dev is
macOS via jj workspaces), but stated here and in `tasks/README.md` so it is a
conscious assumption, not an incidental one. Unlike `run_shell_suites`, the guard
has no name-level belt-and-braces fallback; its sole discriminators are the
`os.access` bit and `SHELL_LIBRARIES` membership.

**Extensionless extra.** The guard must treat `accelerator-visualiser` (appended
by `_EXTRA_SHELL_SOURCES`, no `.sh` extension) like any other source: it is an
off-list entrypoint and must be `755`. No special-casing is needed — it flows
through the `rel not in SHELL_LIBRARIES` branch — but `test_flags_extensionless_entrypoint`
(§3) pins this so a future path-handling refactor cannot silently drop it.

#### 3. TDD tests for the guard

**File**: `tests/unit/tasks/test_lint.py` (or a sibling `test_exec_bits.py`
following the same conventions)
**Changes**: Write these first (red), then implement §1–§2 to green. Two layers,
mirroring the existing template.

Mocked-`Context` layer (patch `lint.shell_sources`, build a `tmp_path` tree, set
`lint.repo_root` via `monkeypatch`/`mocker` as in `test_integration.py:27-43`).
Two construction guards are mandatory or the tests pass vacuously:

- **Materialise every synthetic path on disk.** Each path the mocked
  `shell_sources` returns must be *written* under the patched `repo_root` at the
  intended `0644`/`0755` mode (the `test_integration.py:27-43` `p.chmod` pattern).
  `os.access` on a non-existent path silently returns `False`, so a test that
  only mocks the list — without creating the files — makes every off-list path
  look like a "missing +x" entrypoint and passes for the wrong reason.
  `test_passes_when_invariant_holds` must assert it genuinely distinguishes an
  executable off-list file from a non-executable on-list file, not uniform
  `False`.
- **Patch `SHELL_LIBRARIES` in the stale-entry test.** With a synthetic source
  list, most *real* `SHELL_LIBRARIES` members are un-enumerated, so the
  stale-entry branch fires incidentally and a bare `pytest.raises(Exit)` passes
  regardless of the logic under test. `test_flags_stale_library_entry` must
  `mocker.patch.object(lint, "SHELL_LIBRARIES", frozenset({"scripts/gone.sh"}))`
  alongside a source list that omits it, and assert the offender message names
  `scripts/gone.sh` specifically. Materialise every *other* synthetic source file
  at an invariant-satisfying mode for the patched one-element membership (i.e. all
  executable, since only `scripts/gone.sh` is on-list and it is absent from the
  source list) so the stale-entry line is the **sole** offender — otherwise the
  test muddily exercises two offender paths at once.

```
- test_passes_when_invariant_holds        # off-list 0755 + on-list 0644 → no Exit
- test_flags_entrypoint_missing_x          # off-list file at 0644 → Exit names it
- test_flags_library_carrying_x            # on-list file at 0755 → Exit names it
- test_flags_stale_library_entry           # SHELL_LIBRARIES path not enumerated → Exit names it
- test_exempts_test_fixtures               # test-fixtures/ at 0644 → NOT flagged
- test_fixture_exemption_scope             # near-miss (test-fixtures-x.sh, test-fixturesX/) at a VIOLATING mode IS flagged → exemption is segment-scoped, not substring
- test_flags_extensionless_entrypoint      # off-list extensionless extra (materialised at 0644) → flagged
- test_fail_closed_on_empty_scope          # shell_sources()==[] → Exit, no silent pass
- test_offender_message_lists_each_file    # multiple offenders all named
- test_offender_message_is_copy_pasteable  # each mode line is `chmod +x/-x <path>` with the executable portion before any `#`, AND carries the per-line "then commit" reminder (guards against the reminder being dropped)
```

Two notes on the edge-case tests above, so they cannot pass for the wrong reason:

- `test_fixture_exemption_scope` must materialise the near-miss path at a mode
  that *violates* the invariant (e.g. an off-list non-fixture file at `0644`) and
  assert the guard **does** flag it. A compliant mode produces no `Exit` whether
  or not the match is correctly segment-scoped, so the test would pass even if a
  regression broadened the match to a substring.
- `test_flags_extensionless_entrypoint` relies on the materialise-on-disk rule
  below to write `accelerator-visualiser` at `0644` under the patched
  `repo_root`; an un-materialised path reads as non-executable via the same
  `os.access`-returns-`False`-on-missing path and would pass vacuously.

Anti-vacuous-pass sentinel (per `test_python_coverage.py:133-176`): a test that
injects a known-bad file into the synthetic tree and asserts the guard fails,
proving it is not a no-op; plus a test asserting `shell_sources()` over the real
repo root is non-empty.

Library-list integrity tests (against the real tree):

- **Exact membership** — assert `SHELL_LIBRARIES` equals the reconciled set the
  AC1 re-derivation produced (sorted set-equality). `frozenset` already dedupes,
  so this also catches an accidental duplicate or dropped line in the literal.
- **Each member is enumerated** — assert every `SHELL_LIBRARIES` path appears in
  `shell_sources()` over the real root (stronger than mere `is_file()`; matches
  the `in_scope` domain the guard's stale-entry check now uses).
- **Dual-use guard (regression net for the central trap)** — assert the three
  documented dual-use scripts (`scripts/linkage-parser.sh`,
  `skills/design/inventory-design/scripts/validate-source.sh`,
  `skills/integrations/jira/scripts/jira-fields.sh`) are **absent** from
  `SHELL_LIBRARIES` and are executable on the real tree. This pins the single
  most error-prone classification: a future contributor wrongly adding one to the
  list (stripping a still-path-invoked entrypoint's `+x`) turns the build red.

> The expensive completeness half — proving each member is *only* sourced and
> never path-invoked — stays a documented manual audit (AC1), but the exact-set
> and dual-use assertions above convert the highest-risk slices of it into an
> automated regression net rather than pure documentation.

#### 4. Document the mechanism

**File**: `tasks/README.md` ("Conventions (learn once)", lines 30-45)
**Changes**: Add a single named subsection (heading exactly `### Executable-bit
invariant`, so the guard's offender message and the `SHELL_LIBRARIES` source
comment can both point to it by name) covering:

- (a) **The default and the invariant.** New `.sh` files are entrypoints by
  default — `chmod +x` and commit them; you only touch `SHELL_LIBRARIES` for a
  sourced-only library. The guard enforces *executable ⟺ off the list*.
- (b) **The classification rule is two-part.** A script is on the list iff it is
  **sourced AND never invoked by path**. "Sourced" alone is not sufficient: give
  `jira-fields.sh` as the worked counter-example — it is `source`d by
  `jira-init-flow.sh` yet also invoked `bash …/jira-fields.sh …` in production,
  so it is an **entrypoint** that stays OFF the list at `755`.
- (c) **Maintenance obligations.** A new sourced-only library must be **added**
  to `SHELL_LIBRARIES` (or the guard demands `+x`); a removed or renamed library
  must be **deleted/updated** there (or the stale-entry guard fails).
- (d) **The runner-vs-helper discriminator.** `test-interactive-protocol.sh` is a
  runner → entrypoint → `755`; `test-helpers.sh` is a sourced helper → on the
  list → `644`.
- (e) **Fixtures are a third category.** Scripts under `test-fixtures/**` are
  bash-run migration fixtures (executed via `bash "$f"`, never sourced, never
  path-invoked): the guard exempts them in both directions — they need neither
  `+x` nor a list entry.
- (f) **Working-copy mode.** The guard reads the working-copy mode, so the
  `chmod` must be **committed** to satisfy CI on a fresh checkout. State that the
  guard intentionally enforces working-copy (not VCS-recorded) mode and assumes
  an exec-bit-preserving filesystem — acceptable given the macOS + Linux target
  matrix (see Portability note in §2).

Reference the named subsection (not the bare file) from the `SHELL_LIBRARIES`
source comment. `CLAUDE.md`'s shell section is the secondary home — add a
one-line pointer attached to the **"Conventions and gotchas" shell bullet** (near
"Shell has no autofixer"), where the other actionable shell rules live.

### Success Criteria:

#### Automated Verification:

- [ ] Guard unit tests pass: `uv run pytest tests/unit/tasks/test_lint.py -v`
      (or `test_exec_bits.py`)
- [ ] `mise run build-system:check` exits 0 (ruff + pyrefly clean on the new
      task and constant)
- [ ] `mise run check` exits 0 — confirming the guard is **not** yet wired in
      (it is not run over the real tree this phase, so the phase is green
      independent of Phase 1)
- [ ] The guard is invokable directly and behaves: a deliberately-broken
      `tmp_path` tree makes the behavioural test raise `Exit`.

#### Manual Verification:

- [ ] `tasks/README.md` reads clearly to a contributor classifying a new script
      unaided (runner-vs-helper example present).
- [ ] The `SHELL_LIBRARIES` membership re-audit (AC1 procedure) is recorded —
      each entry has ≥1 source ref and zero path invocations.

---

## Phase 3: Wire the Guard into `mise run check`

### Overview

Register the guard in the `lint:scripts` family so CI enforces it over the real
tree. This is the integration step: it only goes green once Phase 1's
corrections and Phase 2's guard are both present, so it must land last.

### Changes Required:

#### 1. Add the mise task and depend on it

**File**: `mise.toml`
**Changes**: Add a task block beside the existing shell-lint tasks and append it
to the `lint:scripts:check` `depends` list (`mise.toml:234-236`).

```toml
[tasks."lint:scripts:exec-bits:check"]
description = "Enforce exec-bit invariant: a tracked .sh is executable iff not a sourced library"
run = "invoke lint.scripts.exec-bits"
```

```toml
[tasks."lint:scripts:check"]
description = "Run all shell lint checks"
depends = ["lint:scripts:shellcheck:check", "lint:scripts:bashisms:check", "lint:scripts:exec-bits:check"]
```

No `lint:fix` change (preserves "shell has no autofixer"); no `__init__.py` edit
(`exec_bits` auto-registers via `Collection.from_module`).

#### 2. Real-tree integration assertion

**File**: `tests/unit/tasks/test_lint.py` (behavioural layer)
**Changes**: Add a behavioural test that runs the real guard over the actual
repo root (not a `tmp_path`) and asserts it passes — proving Phase 1's
corrections satisfy the invariant the wired guard now enforces. This is the
TDD closing of the loop: the guard is the regression test for the corrections.

### Success Criteria:

#### Automated Verification:

- [ ] The guard runs and passes within the check graph:
      `mise run lint:scripts:exec-bits:check`
- [ ] `mise run scripts:check` exits 0 (guard is in the `lint:scripts:check`
      dependency chain)
- [ ] `mise run check` exits 0 end-to-end (the full CI mirror)
- [ ] Negative path: temporarily `chmod -x` a corrected entrypoint →
      `mise run lint:scripts:exec-bits:check` exits non-zero and names it;
      `chmod +x` a library → same; add a bogus `SHELL_LIBRARIES` path → same.
      (Revert all probes afterwards.)
- [ ] AC5 exclusion: assert no path enumerated by `shell_sources()` contains a
      `node_modules` segment (the stable invariant, robust to Playwright
      reorganising its `node_modules` tree). Keep the deep guarantee at the
      `shell_sources()` unit level (synthetic gitignore tree in
      `test_sources.py`) rather than asserting on a concrete third-party file
      such as `reinstall_chrome_stable_linux.sh`, which can move on a frontend
      dependency bump and make the test flaky.

#### Manual Verification:

- [ ] CI (`mise run scripts:check` in `.github/workflows/main.yml:99-115`) is
      green on the branch.
- [ ] The guard's offender message is actionable (names file + states the
      required `chmod`).

---

## Testing Strategy

### Unit Tests:

- Guard logic over synthetic `tmp_path` trees: invariant-holds (pass), each of
  the three violation directions (entrypoint-missing-`+x`,
  library-carrying-`+x`, stale-list-entry now keyed on enumeration), the
  `test-fixtures/` exemption plus a scope test (matches only the known fixture
  tree), the extensionless-entrypoint case, the copy-pasteable offender message,
  and fail-closed-on-empty-scope.
- Anti-vacuous-pass sentinel (guard fails on a known-bad injected file;
  `shell_sources()` over the real root is non-empty).
- `SHELL_LIBRARIES` integrity over the real tree: exact set-equality against the
  reconciled AC1 set, every member enumerated by `shell_sources()`, and the
  dual-use regression net (the three dual-use scripts absent from the list and
  executable).

### Integration Tests:

- Real-tree behavioural test: the wired guard passes over the corrected repo
  (Phase 3).
- `node_modules/` exclusion spot-check (AC5).

### Manual Testing Steps:

1. From within the build-system workspace, `chmod -x` one corrected entrypoint,
   run `mise run lint:scripts:exec-bits:check`, confirm it fails naming the file,
   then restore.
2. Repeat with `chmod +x` on a library and with a bogus list entry.
3. Bare-path-invoke a previously-broken entrypoint (e.g. a jira `*-flow.sh`) to
   confirm it now executes.

## Performance Considerations

Negligible. The guard is a single `os.walk` (via `shell_sources()`, already run
by the sibling shell-lint tasks) plus per-file `os.access` calls — far cheaper
than ShellCheck. No new process spawns.

## Migration Notes

- **Mode changes propagate automatically.** The exec bit is a tracked VCS
  attribute; the committed `chmod`s reach every checkout and CI run on the next
  pull. No data migration.
- **Escalation trigger fired (release/packaging owner).** Per work-item decision
  and the research, the 6 jira `*-flow.sh` and the `work-item-*` sync scripts are
  **packaged, bare-path-invoked entrypoints that were shipping without `+x`** —
  the strict 0106 case, which the work item names as the trigger to raise
  priority to high and notify the release/packaging owner. **Action:** flag this
  to the release/packaging owner and consider whether a patch release is
  warranted now that the fix is in hand; track it as a follow-up rather than
  inline narrative. This plan delivers the fix regardless of the priority label.

## References

- Original work item: `meta/work/0122-audit-and-correct-missing-executable-bits-on-skill-entrypoint-scripts.md`
- Related research: `meta/research/codebase/2026-06-20-0122-executable-bits-skill-entrypoint-scripts.md`
- Guard idiom to mirror: `tasks/lint/scripts.py:13-47`
- Mandated enumeration: `tasks/shared/sources.py:60-100` (`_keep` `:29-37`,
  `_EXTRA_SHELL_SOURCES` `:55-57`)
- Auto-registration: `tasks/__init__.py:69-85`
- Wiring points: `mise.toml:226-240` (`lint:scripts:*`), `:334-356`
  (`lint:check`/`check`/`default`); CI `.github/workflows/main.yml:99-115`
- Exec-bit discovery precedent: `tasks/test/helpers.py:17-44`
- Test templates: `tests/unit/tasks/test_lint.py:14-158`,
  `tests/unit/tasks/test_integration.py:27-43`,
  `tests/unit/tasks/shared/test_sources.py:28-67`,
  `tests/unit/tasks/test_python_coverage.py:133-176`
- Convention home: `tasks/README.md:30-45`
- Related work items: 0106 (`meta/work/0106-invoke-plugin-scripts-by-bare-path.md`),
  0098 (`meta/work/0098-repo-wide-linting-formatting-static-analysis.md`),
  0107 (`meta/work/0107-lint-skill-body-script-invocations.md`)
