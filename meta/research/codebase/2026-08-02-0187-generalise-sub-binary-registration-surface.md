---
type: "codebase-research"
id: "2026-08-02-0187-generalise-sub-binary-registration-surface"
title: "Research: Generalise the Sub-Binary Registration Surface (0187)"
date: "2026-08-02T21:22:36+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0187"
parent: "work-item:0187"
topic: "Generalising validate_dispatch_coherence over dispatch tokens, parameterising the release-stage builders, and documenting the sub-binary registration surface"
tags: ["research", "codebase", "build-system", "distribution", "rust", "skills", "tasks"]
revision: "4ccf888b9b6c29f4b8b752cf5ba6f76aeef2f610"
repository: "accelerator"
last_updated: "2026-08-02T21:22:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Generalise the Sub-Binary Registration Surface (0187)

**Date**: 2026-08-02 21:22 UTC
**Author**: Toby Clemson
**Git Commit**: `4ccf888b9b6c29f4b8b752cf5ba6f76aeef2f610`
**Branch**: detached (workspace `visualisation-system`)
**Repository**: accelerator

## Research Question

What does the codebase actually look like for the work described in
`meta/work/0187-generalise-sub-binary-registration-surface.md` — generalising
`validate_dispatch_coherence` over dispatch tokens, adding
parameter-with-default seams to the three release-stage builders, renaming
`_BARE_LAUNCHER`, and documenting a ten-point registration checklist in
`tasks/README.md`?

## Summary

The task is implementable broadly as written, and its blocker is discharged
(0168 is `status: done`). But three of its specified mechanisms **do not work as
literally written**, two of them proven empirically here:

1. **The permission probe is wrong.** `covered_by("${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser", "${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser *")`
   is **`False`**. The probe the Requirements specify fails condition 1 against
   the visualiser's real rule, so a literal implementation reports the
   visualiser as *unbound* and fails the release — directly contradicting the
   "The visualiser binding still passes" criterion. The probe needs a trailing
   space (or a sentinel argument). Review-1 flagged the *inversion* hazard but
   never settled the probe literal; this is the residual half of that finding.

2. **A module-level import of `tasks.lint.skill_permissions` from
   `tasks/build.py` is a hard circular import** that breaks the entire build
   system (`import tasks`, `import tasks.build`, `import tasks.lint`,
   `import tasks.manifest` all raise `ImportError`). `tasks/lint/__init__.py`
   imports `vendor_shims`, which imports `from tasks.build import ...`. Two
   remedies verified working.

3. **There are three SLSA attestation blocks, not one.** Checklist point 9 cites
   `.github/workflows/main.yml:423-425`; the same visualiser-shaped
   `subject-path` pair appears at `:420-425`, `:531-536` and `:553-558`.

Beyond those, the work item's registration enumeration is accurate but
**incomplete in one place**: `debug_archive_path` (`tasks/shared/paths.py:79-80`)
is hardwired to `visualiser` and `_release_uploads` appends debug archives
unconditionally per platform (`tasks/github.py:224`) — a fourth visualiser-shaped
release stage the item's Assumptions do not name. That hardwiring is precisely
*why* the visualiser-shaped SLSA line exists.

The mechanical facts otherwise check out: `.gitignore:44` really is `bin/visualiser-*`
(bare token, confirming point 5's `bin/<token>-*`); `test_manifest.py:38` really
is stale and `test_manifest_contract.py:16` really is the Python co-reader; the
launcher's dispatch path really is fully generic over tokens; and all five
hand-off dependency edges plus the epic-0136 annotation are present and correct.

## Detailed Findings

### 1. The permission probe — the specification defect that would fail the release

`covered_by` (`tasks/lint/skill_permissions.py:106-113`) appends `*` only when
the pattern does not already end in `*`:

```python
def covered_by(command: str, pattern: str) -> bool:
    glob = pattern if pattern.endswith("*") else pattern + "*"
    return fnmatch.fnmatchcase(command, glob)
```

The visualiser's real rule (`skills/visualisation/visualise/SKILL.md:8`) is
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser *)`. It already ends in
`*`, so the glob is used verbatim — and the **space before the `*` is a literal
that must be matched**. Empirically:

| probe | vs `…/accelerator visualiser *` | vs `…/accelerator config *` | vs ancestor `…/accelerator *` |
|---|---|---|---|
| `${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser` (as specified) | **False** | False | True |
| `${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser ` (trailing space) | True | False | True |
| `${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser zz-probe-zz` | True | False | True |

`covered_by(BARE_LAUNCHER, rule)` is `False` for both real rules and `True` for
the ancestor glob, so condition 2 behaves as the item describes in every case.
The defect is confined to condition 1's probe.

Either corrected probe works. The trailing-space form is minimal; the
sentinel-argument form (`… <token> zz-probe-zz`) mirrors the existing
`_BARE_LAUNCHER` design at `:42-44` and is arguably the more idiomatic sibling.
Both also correctly cover the no-glob rule shape `Bash(…/accelerator visualiser)`,
because `covered_by` appends the `*` in that case.

Note the corrected probe is *not* the real invocation string. The real
invocation (`SKILL.md:30`) is
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser --owner-pid $PPID ${ARGUMENTS:-start}`,
which also matches — but using it as the probe would make the guard test the
skill's own command rather than the token, and would not generalise to a token
whose consuming skill has not been written yet.

### 2. Circular import — a module-level reuse import breaks the whole build system

`tasks/lint/vendor_shims.py:3`:

```python
from tasks.build import _assert_magic_bytes, vendor_shim_marker_digest
```

and `tasks/lint/__init__.py:1-14` imports `vendor_shims` eagerly. So adding
`from tasks.lint.skill_permissions import …` at module scope in
`tasks/build.py` produces:

```
ImportError: cannot import name '_assert_magic_bytes' from partially
initialized module 'tasks.build' (most likely due to a circular import)
```

Verified: this fires for `import tasks.build`, `import tasks`, `import tasks.lint`
**and** `import tasks.manifest` — i.e. every entry point, including `mise run`.
The import in `build.py` sits at lines 11-32, well before `_assert_magic_bytes`
is defined at `:104`, so there is no reordering that saves it.

Two remedies were tested and both work cleanly (all four import paths OK, and
the guard still runs):

- **Remedy A — function-local import.** Move
  `from tasks.lint.skill_permissions import …` inside
  `validate_dispatch_coherence`. Smallest diff; keeps the guard in
  `tasks/build.py` where every acceptance criterion anchors it. A source-scan
  test asserting the imports still sees them (the scan is regex-over-text, per
  the house idiom — see §6).
- **Remedy B — extract to a new module.** Put the guard in e.g.
  `tasks/dispatch.py`, have `tasks/manifest.py` import it from there. Verified
  no cycle. This *also* dissolves review-1's unresolved "what counts as the
  guard's helpers?" scoping problem for the no-literal-`visualiser` source scan
  — a dedicated module makes it a whole-file scan.

Remedy B's cost: it moves the guard away from the file the criteria name, and
the "`tests/unit/tasks/test_build.py` covers this pass path and every fail path"
criterion would need re-homing. Remedy A's cost: the source scan must still be
hand-scoped to the guard function plus its named helpers, because
`tasks/build.py` legitimately retains `visualiser` literals at `:213`, `:237`,
`:265`, `:292`, `:295`, `:306`, `:309`, `:500`, `:507`.

### 3. `validate_dispatch_coherence` today, and what its tests pin

`tasks/build.py:189-208` — the guard, called from `tasks/manifest.py:138`
inside `emit_manifest`, so it runs on every release:

```python
root = repo_root or REPO_ROOT
skill = (root / _VISUALISE_SKILL_RELATIVE).read_text()
invokes = "accelerator visualiser" in skill
dispatched = "visualiser" in DISPATCHED_SUBBINARIES
if invokes != dispatched:
    raise DispatchCoherenceError(...)
```

`_VISUALISE_SKILL_RELATIVE` (`tasks/build.py:35`) has exactly two occurrences,
both in `build.py` (`:35`, `:199`) — nothing in `tests/` references it, so
removal is clean.

The bare-substring defect the item names is real: `accelerator visualiser`
appears in `SKILL.md` at `:46` and `:160` as prose as well as at `:30` as the
real invocation (`:8` is the `allowed-tools` rule).

Existing coverage — `tests/unit/tasks/test_build.py:118-148`,
`TestValidateDispatchCoherence`:

- `test_coherent_repo_passes` calls `tb.validate_dispatch_coherence()` against
  the real repo with no arguments. This is already the "real constants" path and
  should survive as-is.
- The other three tests each `mocker.patch.object(tb, "DISPATCHED_SUBBINARIES", …)`
  and hand-seed a single `skills/visualisation/visualise/SKILL.md` into bare
  `tmp_path` with a body of `"run \`accelerator visualiser start\`"` — a
  backticked string that the *new* definition of an invocation (a `!`-preprocessor
  command recognised by `is_plugin_invocation`) deliberately does **not** match.
  All three must be rewritten, not merely re-parameterised.
- `test_both_absent_is_coherent` (`:145-148`) asserts an **empty collection
  passes**. Under the new anti-vacuity rule it must assert the opposite. This is
  a direct behavioural reversal of a committed test, worth calling out in the
  plan.

`tests/conftest.py:6-30` (`fake_repo_tree`, the only conftest in the tree) writes
`.claude-plugin/plugin.json` and a `cli/` workspace but **no `skills/` tree**, so
the guard's tests will keep their own seeding helper rather than adopting it.

### 4. What the skills tree actually contains

Applying the proposed invocation definition across every `skills/**/SKILL.md`
today yields exactly two launcher tokens:

| token | invocations | skills |
|---|---|---|
| `config` | 204 | 45 |
| `visualiser` | 1 | 1 |

The only other `${CLAUDE_PLUGIN_ROOT}/`-prefixed preprocessor commands are three
`scripts/*.sh` paths (`config-read-browser-executor.sh`, `vcs-status.sh`,
`vcs-log.sh`) — not launcher invocations. So the invocation→registration
direction has a clean baseline: `config` is a built-in, `visualiser` is
registered, nothing else exists.

Two edge cases the Requirements do not cover:

- **A skill declaring bare `Bash`** (`has_bare_bash`, `skill_permissions.py:82-84`)
  has zero `Bash(...)` rules, so it can never satisfy the two-condition test and
  would count as *unbound*. That is arguably correct — bare `Bash` is maximally
  over-broad, exactly what condition 2 exists to reject — but it is unstated. No
  skill currently invoking the visualiser is affected.
- **A launcher invocation with no token at all** (`…/bin/accelerator` alone).
  None exists today; the extractor needs a defined behaviour anyway.

The token extractor must also match `…/bin/accelerator` followed by a space or
end-of-string, not as a bare prefix — `accelerator-verify-*` binaries exist in
`bin/`, and a sloppy `startswith` would mis-tokenise.

### 5. The release-stage builders and their injectable shape

- **Signing** — `tasks/signing.py:50-73`. The extraction the item wants is
  local: the sub-binary half of `expected` (`:62-66`) is a two-line
  comprehension, iterated platform-major
  (`for _triple, platform in TARGETS for token in DISPATCHED_SUBBINARIES`).
  The "4 unsigned binary paths" default-case figure in the acceptance criteria
  implies the helper returns **only** the sub-binary paths (1 token × 4 targets),
  not the four launcher paths. The Test-seams *fallback* looks unnecessary — the
  extraction does not touch the signing flow.
  `sign_staged_binaries` has **no direct unit test** today; it is only mocked at
  `tests/integration/tasks/test_release.py:134`.
- **Uploads** — `tasks/github.py:218-235`. The sub-binary loop (`:230-234`) is
  already token-generic. But `:224` appends `debug_archive_path(platform)`
  unconditionally, which is visualiser-hardwired (see §7). The criterion's "8
  upload entries … every one naming `visualiser`" therefore does not describe the
  whole return value: the full list is 22 entries, of which 12 name `visualiser`
  (4 debug archives + 8 sub-binary asset/`.minisig` pairs). The test must filter
  to the sub-binary-derived subset, or the sub-binary loop needs its own
  extracted helper.
- **Re-verification** — `tasks/github.py:270-293`. Its early return reads the
  module constant directly (`if not DISPATCHED_SUBBINARIES: return []`, `:271`)
  and must become the parameter. It also reads `RELEASE_MANIFEST` (`:273`), which
  is *not* in the item's specified signature change — supplying the "in-test
  manifest carrying both tokens" needs either an extra parameter or
  `mocker.patch.object(gh, "RELEASE_MANIFEST", …)`. The latter has a direct
  precedent at `tests/integration/tasks/test_github.py:271`.

`_release_uploads` and `_subbinary_reverifies` have **no direct tests**; they are
covered only transitively by `TestUploadAndVerifyRelease`, whose
`assert len(uploads) == 22` (`test_github.py:326`) is a hard-coded shape pin.
`test_github.py:403-433` is the existing precedent for injecting a second token
(`mocker.patch.object(gh, "DISPATCHED_SUBBINARIES", ("foo",))`).

Note these live in `tests/integration/tasks/`, not `tests/unit/tasks/` — the new
assertions are pure list-derivation and could go either way, but the existing
neighbours are integration.

### 6. Test idioms available (and the three that would be new)

From `tests/unit/tasks/`:

- **Source scans** are regex over `Path.read_text()`, never `ast`. Canonical
  example `tests/unit/tasks/test_bootstrap_coverage.py:54-74` — module-level
  `Path` constants, `re.findall`, set-equality with an explanatory assert
  message. `tests/unit/tasks/test_manifest_contract.py:51-57` extracts a Rust
  constant the same way.
- **Guard-module scans** follow a two-part shape: the logic lives in
  `tasks/lint/<guard>.py` exposing `violations(root) -> list[str]`, and the test
  pairs synthetic `tmp_path` positives with a real-tree
  `violations(REPO_ROOT) == []` (`test_store_duplication.py:24-64`,
  `test_call_site_migration.py:16-59`, `test_claude_coupling.py`).
- **Spies** use `pytest-mock`'s `mocker.patch.object` with
  `assert_called_once_with` (`tests/integration/tasks/test_release.py:124-142`);
  `monkeypatch` is reserved for env vars.
- **New idioms this task introduces**: `inspect.signature` default assertions
  (zero precedent in `tests/`), an `ast`-free but structurally-parsing docs test
  over `tasks/README.md` (zero precedent — no test reads any real markdown), and
  a positive *import* assertion. Ruff's `tests/**` per-file-ignores
  (`pyproject.toml:106-111`) allow `SLF001`, so private-symbol access in tests is
  fine.

Runner: `mise run test:unit:tasks` (`mise.toml:150-153`) — there is **no**
`test:unit:build-system` task. Single test:
`uv run pytest tests/unit/tasks/test_build.py::TestValidateDispatchCoherence -v`.

`tests/unit/tasks/test_skill_permissions.py:74-79` is the only `_BARE_LAUNCHER`
coverage and asserts on the message substring `"without a subcommand"`, never
importing the constant — so the rename touches only
`tasks/lint/skill_permissions.py:42` and `:187`.

### 7. Registration points — verification of the ten-point list

| # | Item's claim | Verified |
|---|---|---|
| 1 | `DISPATCHED_SUBBINARIES` at `tasks/shared/paths.py:25` | ✅ exact |
| 2 | `_SUBBINARY_MANIFESTS` at `tasks/manifest.py:51-53` | ✅ exact; default at `:56-57` |
| 3 | `package.description` (`tasks/manifest.py:60-66`), `version.workspace` (version check at `tasks/build.py:74-101`) | ✅ both exact; `cli/visualiser/server/Cargo.toml:1-8` carries both |
| 4 | `cli/Cargo.toml` members | ✅ `cli/Cargo.toml:2-4`, 12 members on one line |
| 5 | `.gitignore` (`bin/<token>-*`) | ✅ `.gitignore:44` is literally `bin/visualiser-*` — bare token, not `accelerator-<token>` |
| 6 | fixture + `test_manifest_contract.py:16` + `resolve/manifest.rs:135`; `test_manifest.py:38` stale | ✅ all three exact. `test_manifest.py:38` reads `manifest.schema.json`, not the example — the stale-reference correction is right |
| 7 | skill binding or exemption | ✅ (new mechanism) |
| 8 | cross-compile staging `tasks/build.py:37`, `:290-331`; `_assert_static_elf` `:132-159` | ⚠️ `:37` is stale (it is `_CLI_RELEASE_BINARIES`); `server_cross_compile` is `:290-309`, `cli_cross_compile` is `:312-331`. `_assert_static_elf` `:132-159` exact — but it is called **only** from `cli_cross_compile:330`, never for sub-binaries |
| 9 | SLSA `.github/workflows/main.yml:423-425` | ⚠️ **three blocks**, not one: `:420-425`, `:531-536`, `:553-558`, each with the same two `subject-path` lines |
| 10 | launcher built-in lockstep | ⚠️ names no artefact; see below |

Point 8's `_assert_static_elf` note deserves care: sub-binaries currently get
`_assert_magic_bytes` (`:307`) and `_assert_no_e2e_insecure` (`:308`) but **not**
the static-ELF assertion. The checklist should say what a new musl sub-binary
owes, not just cite the helper.

Point 10 can now name its artefacts. Adding a launcher built-in touches five
sites, four compile-enforced and one not:

- `cli/launcher/src/launch/inbound/cli.rs:17-29` — the `Command` enum (source of
  truth: `Version`, `Config`, `#[command(external_subcommand)] External`)
- `cli/launcher/src/launch/mod.rs:183-197` — exhaustive match
- `cli/launcher/src/main.rs:139-144`, `:182-189` — exhaustive matches
- **`cli/launcher/src/main.rs:102-108`** — `is_root_help`, a hardcoded
  `Some("version" | "config" | "help")` string list that is **not**
  compile-enforced. This is the drift site, and the natural literal for the
  README's mechanical test to pin.

Plus the Python side: whatever constant the generalised guard uses for its
built-in set.

### 8. The visualiser-shaped stage the item does not list

`tasks/shared/paths.py:79-80`:

```python
def debug_archive_path(platform: str, bin_dir: Path = BIN_DIR) -> Path:
    return bin_dir / f"accelerator-visualiser-{platform}.debug.tar.gz"
```

Consumers: `create_debug_archives` (`tasks/build.py:498-510`, itself calling
`subbinary_asset_path("visualiser", platform)` at `:507`) and
`_release_uploads` (`tasks/github.py:224`, unconditional per platform).

This is the reason SLSA point 9 exists at all — the archives are staged under
`skills/visualisation/visualise/bin/`, which is exactly the
`skills/visualisation/visualise/bin/accelerator-visualiser-*` `subject-path`
line the item calls visualiser-shaped. The two facts are one fact.

Under the item's own Assumptions rule ("absorb the fix here when it is local to
the token loop"), this is **not** local — it is outside the token loop in both
`_release_uploads` and `create_debug_archives`, and it is `mise`-wired
(`build:debug-archives` depends on `build:server:cross-compile`). So the correct
disposition is a sibling task or an explicit "No action when the sub-binary
ships no debug archive" clause in the checklist, not silent absorption.

Similarly, `server_cross_compile` (`tasks/build.py:290-309`) is the *only*
producer of a staged sub-binary asset and is entirely visualiser-specific
(single `CARGO_TOML`, artefact name `accelerator-visualiser`). Point 8 correctly
frames this as an author action per sub-binary rather than something to
generalise here.

### 9. Token naming constraints — wider than "no underscore"

`derive_override_var` (`cli/launcher/src/launch/core.rs:215-240`) is the real
gate, and it runs before any network or cache path (`src/launch/outbound/mod.rs:21-33`,
called first from `src/main.rs:61-63`). A token must:

- start with an ASCII **letter** (rejects empty and leading digit);
- contain only ASCII alphanumerics and `-` (this is what rejects `_`);
- the underscore rejection is for **injectivity** — `-` maps to `_`, so
  `frob-thing` and `frob_thing` would collide on `ACCELERATOR_FROB_THING_BIN`
  (documented at `core.rs:207-209`).

Error: `ResolutionError::InvalidOverrideName` (`core.rs:40-43`, `:99-102`),
mapped to `kernel::Error::Failed`, exit 1.

Two further constraints the item does not state:

- **Asset-name collision.** `subbinary_asset_path` yields
  `accelerator-<token>-<platform>` and `cli_binary_path` stages
  `accelerator-verify-<platform>` into the same `dist/release/` directory, so a
  token of `verify` would collide with the vendored shim. `launcher` is
  similarly reserved by `.gitignore:42` (`bin/accelerator-launcher-*`).
- **cargo-pup whole-crate matching** — already covered by the item's
  `accelerator-<token>` package-naming constraint, sourced from research §8's
  `vcs_domain_imports_only_permitted` analysis.

### 10. The launcher is already fully generic

Worth recording because it bounds the task: there is **zero** production
coupling to `visualiser` in the launcher's dispatch path. Resolution keys purely
on the token string (`resolve/mod.rs:143-146`,
`format!("accelerator-{name}-{}", platform)`), the manifest's `binaries` map is
the de-facto registry (`manifest.rs:31`, a `BTreeMap<String, BinaryEntry>`), a
missing key is `AssetNotFound`, and the help section for external subcommands is
synthesised from the manifest at runtime (`help.rs:14-31`). The only `visualiser`
literals under `cli/launcher/src/` are in a `#[cfg(test)]` block
(`resolve/manifest.rs:146,148`) and in the unrelated `config` domain
(`config_command/core/dump.rs:70`).

`manifest.schema.json` also has no token allowlist (`binaries` is
`additionalProperties`, `:18-22`), though the platform-alias `enum` at `:37` is
a closed set.

### 11. Docs surface not covered by the ten points

Three user-facing tables would go stale for a second sub-binary and are not in
the checklist:

- `docs/visualiser.md:59-73` — the override-variable table, stating
  `ACCELERATOR_VISUALISER_BIN` as a literal rather than the
  `ACCELERATOR_<TOKEN>_BIN` pattern;
- `docs/visualiser.md:42-55` — the download flow, phrased in the singular;
- `docs/internals.md:202-205` — the env-var / trust-root table.

`tasks/README.md` has no distribution section at all today; its only touch is a
passing analogy at `:127`. Whether the checklist should own these is a scope
call for the plan.

### 12. Blocker and hand-off state — all verified green

- **0168 is `status: done`** (`meta/work/0168-fold-visualiser-into-cli-workspace.md:8`),
  closed by commit `abfaf60bd9`. The blocker's first discharge route is met, so
  the Open Question resolves and the "Visualiser manifest path re-verification"
  Validation Results slot is *not required*. The work item's Dependencies prose
  at `:382-383` ("its work item is still `ready`") is stale by one day.
  0168 also pinned `cli/visualiser/server` by acceptance criterion, and none of
  its three open questions can relocate the crate manifest.
- **`blocked_by: work-item:0187`** present on 0170 (`:12`), 0171 (`:12`),
  0172 (`:12`) and 0173 (`:12`); 0169 (`:12`) too.
- **`blocks: ["work-item:0187"]`** present on 0168 (`:13`).
- **Epic 0136's annotation** (`meta/work/0136-migrate-shell-scripts-to-rust-cli.md:65-70`)
  reads exactly as the criterion requires, including 0172 in the unblocks list
  and the two 0168 discharge routes.

The acceptance-time grep re-verification passes today.

## Code References

- `tasks/build.py:35` — `_VISUALISE_SKILL_RELATIVE` (removed by this task; only other use `:199`)
- `tasks/build.py:189-208` — `validate_dispatch_coherence`
- `tasks/build.py:74-101` — `_pinned_member_versions`, the *version*-coherence check (checklist point 3, not the dispatch guard)
- `tasks/build.py:132-159` — `_assert_static_elf`; called only from `cli_cross_compile:330`
- `tasks/build.py:290-309` — `server_cross_compile` (item claims `:37`/`:290-331`; both drifted)
- `tasks/build.py:498-510` — `create_debug_archives`, visualiser-hardwired
- `tasks/manifest.py:51-57` — `_SUBBINARY_MANIFESTS` and its `cli/<token>/Cargo.toml` default
- `tasks/manifest.py:60-66` — `_read_description`, the `package.description` requirement
- `tasks/manifest.py:138` — the guard's release call site, inside `emit_manifest`
- `tasks/shared/paths.py:25` — `DISPATCHED_SUBBINARIES`; sibling site for `SKILL_EXEMPT_SUBBINARIES`
- `tasks/shared/paths.py:60-70` — `subbinary_asset_path` (generic)
- `tasks/shared/paths.py:79-80` — `debug_archive_path` (visualiser-hardwired)
- `tasks/shared/errors.py:10` — `DispatchCoherenceError`
- `tasks/signing.py:50-73` — `sign_staged_binaries`, expected-set construction at `:62-66`
- `tasks/github.py:218-235` — `_release_uploads`; `:224` is the unconditional debug-archive append
- `tasks/github.py:270-293` — `_subbinary_reverifies`; `:271` reads the module constant, `:273` reads `RELEASE_MANIFEST`
- `tasks/lint/skill_permissions.py:42-44` — `_BARE_LAUNCHER` (rename target)
- `tasks/lint/skill_permissions.py:57` — `PLUGIN_PREFIX`, already public with a "second copy could desynchronise" comment
- `tasks/lint/skill_permissions.py:106-113` — `covered_by`
- `tasks/lint/skill_permissions.py:183-188` — the existing condition-2-alone over-broad-rule check
- `tasks/lint/vendor_shims.py:3` — the import that closes the circle
- `tasks/lint/__init__.py:1-14` — eager import of `vendor_shims`
- `skills/visualisation/visualise/SKILL.md:8` — the passing rule; `:30` the invocation; `:46`, `:160` the prose that satisfies today's substring match
- `cli/launcher/src/launch/inbound/cli.rs:17-29` — the built-in `Command` enum
- `cli/launcher/src/main.rs:102-108` — `is_root_help`, the non-compile-enforced built-in list
- `cli/launcher/src/launch/core.rs:215-240` — `derive_override_var`
- `cli/launcher/src/launch/outbound/resolve/manifest.rs:135` — the `include_str!` co-reader
- `cli/launcher/tests/fixtures/manifest.example.json` — the golden fixture (28 lines, one `visualiser` entry)
- `.gitignore:44` — `bin/visualiser-*`
- `.github/workflows/main.yml:420-425`, `:531-536`, `:553-558` — the three SLSA blocks
- `tests/unit/tasks/test_build.py:118-148` — `TestValidateDispatchCoherence`
- `tests/unit/tasks/test_manifest_contract.py:14-21` — the fixture path constants
- `tests/unit/tasks/test_bootstrap_coverage.py:54-74` — the source-scan idiom
- `tests/unit/tasks/test_skill_permissions.py:74-79` — the only `_BARE_LAUNCHER`-adjacent test
- `tests/integration/tasks/test_github.py:251-308` — `_setup_release`; `:326` the `== 22` pin; `:403-433` the second-token precedent
- `tests/conftest.py:6-30` — `fake_repo_tree` (no `skills/` tree)
- `mise.toml:150-153` — `test:unit:tasks`

## Architecture Insights

- **The manifest is the registry; `DISPATCHED_SUBBINARIES` is the producer's
  view of it.** The launcher has no allowlist — it dispatches any token and
  fails with `AssetNotFound` if the signed manifest lacks the key. So every
  registration point that matters is on the *producer* side, which is why the
  whole surface lives in `tasks/` and why documenting it there is the right
  home.
- **Registration is multi-step because the constraints are heterogeneous.**
  Cargo names, gitignore patterns, a shared golden fixture with a Rust and a
  Python co-reader, a workflow glob, and a skill's permission grammar do not
  share a derivation. The item's decision to document rather than collapse is
  well-founded — see review-1's costing of the collapse as "a build-system
  refactor several times the size of the described task".
- **The reuse requirement is the interesting design constraint.** Making a
  release-gating guard a consumer of a lint module's parsing surface is the
  coupling the rename makes explicit. The circular-import finding shows that
  coupling is not free even mechanically — the dependency direction
  `build → lint` is currently blocked by `lint → build`, which is itself a hint
  that the shared parsing might eventually belong in `tasks/shared/`. The item
  explicitly requires importing from `tasks.lint.skill_permissions`, so that
  refactor is out of scope, but it is the natural follow-up if a third consumer
  appears.
- **Anti-vacuity is the recurring theme of this codebase's guards.**
  `test_python_coverage.py` pins ruff/pyrefly scope as exact sets *and* runs a
  sentinel probe; `test_claude_coupling.py` has an explicit `TestFailClosed`
  floor class; `skill_permissions.py:39` pins `EXPECTED_INJECTION_SKILLS = 42` by
  equality. The empty-collection requirement is squarely in that tradition, and
  the existing `test_both_absent_is_coherent` is the outlier.

## Historical Context

- `meta/reviews/work/0187-generalise-sub-binary-registration-surface-review-1.md`
  — five lenses, five passes, verdict APPROVE set by the author after a pass-5
  lens verdict of REVISE. Four majors were **accepted as-is**, and they are
  effectively unwritten requirements: (i) "an imperative action line" is not
  mechanically decidable; (ii) the source-scan region "its helpers" is
  undefined and must exclude `build.py`'s legitimate visualiser material;
  (iii) `blocked_by: 0168` is a confirmation gate, prose is operative;
  (iv) the 0168 re-verification should be read as covering checklist points 5
  and 9 too. The review also records the `covered_by` inversion finding (pass 3)
  — but never settles the probe literal, which is the gap §1 above closes.
- `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §8 — the eight-row provenance table. Its SLSA row ("already cover
  `accelerator-*`") is wrong and unlabelled as such by the item; its
  `test_manifest.py:38` row is stale, and the item's correction is right.
- `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md:811-821`
  — the pass-4 decomposition that carved 0187 out as item 2 of four.
- `meta/work/0165-multi-binary-distribution-and-release-pipeline.md` — delivered
  the pipeline but **never names** `DISPATCHED_SUBBINARIES`,
  `validate_dispatch_coherence` or `_SUBBINARY_MANIFESTS`. The
  token-parameterisation 0187 builds on is emergent, never a stated contract —
  which is the strongest argument for discharging it by test.
- `meta/work/0168-fold-visualiser-into-cli-workspace.md:181-193` — 0168's
  reciprocal bullet, including the post-landing contingency (if residual scope
  moves the crate path after 0187 lands, update `_SUBBINARY_MANIFESTS` and the
  README checklist in the same change).
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
  — the governing decision.
- `meta/research/issues/2026-06-10-bash-prefix-defeats-skill-allowed-tools-permission.md`
  — the RCA behind `covered_by`'s prefix-glob semantics.

## Related Research

- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md`
- `meta/research/codebase/2026-07-06-0165-multi-binary-distribution-release-pipeline.md`
- `meta/research/codebase/2026-07-23-0168-fold-visualiser-into-cli-workspace.md`
- `meta/research/codebase/2026-07-19-0167-config-command-and-invocation-contract-migration.md`
- `meta/research/codebase/2026-07-27-0182-plugin-root-self-location-implementation-surface.md`

## Open Questions

1. **Which circular-import remedy?** Function-local import in `tasks/build.py`
   (minimal, keeps every criterion's anchor) versus extracting the guard to a
   new module (also solves the "its helpers" source-scan scoping problem, but
   moves the guard away from the file the criteria name). §2 has both verified.
2. **Which corrected probe literal?** Trailing space, or a
   `zz-…-zz` sentinel argument mirroring `_BARE_LAUNCHER`. Both verified
   equivalent for every rule shape in the tree.
3. **Does the debug-archive hardwiring get absorbed, deferred, or documented?**
   §8 argues it is outside the token loop, so the item's own Assumptions push it
   to a sibling task — but checklist point 9 already half-documents its
   consequence, so an explicit "No action when the sub-binary ships no debug
   archive" clause may be the cheaper closure.
4. **Should checklist point 9 name all three SLSA blocks?** The criterion's
   literal-string list pins `dist/release/accelerator-*` but nothing pins the
   *count* of blocks, so an author updating one and missing two would still pass
   the mechanical test.
5. **`##` or `###` for the README section?** The acceptance criterion says
   `## Registering a dispatched sub-binary`; the file's own tiering would put a
   deep-dive at `###` under "Conventions (learn once)" alongside the
   executable-bit invariant. The GitHub anchor the sibling work items link to is
   identical either way, so this is a house-style call, not a linkage risk.
6. **Where do the three builder tests live?** Their neighbours
   (`_release_uploads`, `_subbinary_reverifies`) are covered in
   `tests/integration/tasks/test_github.py`, but the new assertions are pure and
   need no network — `tests/unit/tasks/` would be the honest home, at the cost
   of splitting coverage of one module across two suites.
7. **Do `docs/visualiser.md` and `docs/internals.md` join the checklist?**
   §11 lists three user-facing tables that go stale for a second sub-binary and
   are in none of the ten points.
