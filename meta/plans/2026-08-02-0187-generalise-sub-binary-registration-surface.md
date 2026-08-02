---
type: plan
id: "2026-08-02-0187-generalise-sub-binary-registration-surface"
title: "Generalise the Sub-Binary Registration Surface Implementation Plan"
date: "2026-08-02T22:12:19+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0187"
parent: "work-item:0187"
derived_from: ["codebase-research:2026-08-02-0187-generalise-sub-binary-registration-surface"]
relates_to: ["plan-review:2026-08-02-0187-generalise-sub-binary-registration-surface-review-1"]
tags: [build-system, distribution, rust]
revision: "d7b55d39a690ce91e887cf5e464320cb08c038b2"
repository: "accelerator"
last_updated: "2026-08-03T11:39:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Generalise the Sub-Binary Registration Surface Implementation Plan

## Overview

Extract the SKILL.md parsing primitives into a dependency-free
`tasks/shared/skill_parsing.py`, make `validate_dispatch_coherence` generic over
the dispatch token on top of them, remove every visualiser-shaped assumption
from the release path, give the release-stage builders injected token
collections, and document the registration surface in `tasks/README.md` as a
mechanically-checked thirteen-point checklist — so the five epic-0136 stories
that each ship a sub-binary add a token rather than rediscovering the surface.

## Current State Analysis

`accelerator-visualiser` is the only dispatched sub-binary.
`DISPATCHED_SUBBINARIES` (`tasks/shared/paths.py:25`) is the producer's
registry; the launcher has no allowlist at all and dispatches any token the
signed manifest names.

Four visualiser-shaped points exist on the producer side:

1. **`validate_dispatch_coherence`** (`tasks/build.py:189-208`) reads a
   hardcoded `_VISUALISE_SKILL_RELATIVE` (`:35`) and compares `"accelerator
   visualiser" in skill` against `"visualiser" in DISPATCHED_SUBBINARIES`. The
   binding it enforces is unenforced for every other token, and the
   bare-substring match is satisfied by prose
   (`skills/visualisation/visualise/SKILL.md:46`, `:160`) as well as by the real
   invocation (`:30`).
2. **`debug_archive_path`** (`tasks/shared/paths.py:79-80`) is hardwired to
   `visualiser`, as is `create_debug_archives` (`tasks/build.py:498-510`), and
   `_release_uploads` appends an archive unconditionally per platform
   (`tasks/github.py:224`).
3. **SLSA provenance** — three `attest-build-provenance` blocks
   (`.github/workflows/main.yml:423`, `:534`, `:556`), each carrying the same
   `skills/visualisation/visualise/bin/accelerator-visualiser-*` subject-path
   line beside the generic `dist/release/accelerator-*`. This line exists
   *because of* point 2; the two facts are one fact.
4. **`tasks/lint/skill_permissions.py` mixes a pure parsing core with an
   imperative lint shell.** Roughly 60 lines — `frontmatter_bash_rules`,
   `has_bare_bash`, `preprocessor_commands`, `is_plugin_invocation`,
   `covered_by`, `has_metacharacter`, `frontmatter_name` and their regexes —
   depend on nothing but `re` and `fnmatch`, but sit in the same module as
   `@task check`, `violations(root)` and `invoke.Exit`. That mixture, not the
   privacy of `_BARE_LAUNCHER` (`:42-44`), is why the parsing cannot be consumed
   without dragging in `invoke`, `tasks.lint` and — via `vendor_shims` —
   `tasks.build`.

Signing (`tasks/signing.py:50-73`) and the two github builders
(`tasks/github.py:218-235`, `:270-293`) already iterate `DISPATCHED_SUBBINARIES`
and need no behavioural change — only injected collections so that is discharged
by test rather than by inspection.

### Verified empirically during planning

Three of the work item's specified mechanisms do not work as literally written,
and a fourth mechanism drafted during planning was itself wrong. All were
reproduced against the tree:

- **A probe of the bare `${CLAUDE_PLUGIN_ROOT}/bin/accelerator <token>` is
  broken.** `covered_by` (`skill_permissions.py:106-113`) uses a pattern already
  ending in `*` verbatim, so the space before the `*` is a literal that must be
  matched, and the probe returns **`False`** against the visualiser's real
  `Bash(… visualiser *)` rule. A literal implementation reports the visualiser
  as *unbound* and fails the release.
- **A synthetic sentinel-argument probe is wrong in both directions.** It was
  the drafted remedy and does not survive contact with realistic rule shapes:

  | Rule | `covered_by(probe, rule) and not covered_by(BARE_LAUNCHER, rule)` |
  |---|---|
  | `Bash(…/accelerator visualiser *)` | `True` — correct |
  | `Bash(…/accelerator visualiser)` | `True` — correct |
  | `Bash(…/accelerator visualiser start)` | **`False`** — a correctly-scoped rule reported unbound |
  | `Bash(…/accelerator visualiser --owner-pid *)` | **`False`** — likewise |
  | `Bash(…/accelerator *)` | `False` — correct |
  | `Bash(…/accelerator v*)` | **`True`** — pre-authorises `vcs`, `version`, every `v`-prefixed token |
  | `Bash(…/accelerator [a-y]*)` | **`True`** — authorises everything not starting with `z` |

  The false negatives fail the release for a rule the lint task's own message
  invites (`:184-185`, "name 'config' (or the specific subcommand)"). The false
  positives certify an over-broad rule as scoped, because `covered_by` delegates
  to `fnmatch` (so `?`, `[seq]` and `[!seq]` are honoured) and the only thing
  separating the two conditions is the sentinel's leading `z`. Phase 2 therefore
  probes with the **actual invoked command** and adds a **structural check on
  the rule's token segment**; see the decisions table.
- **A module-level `from tasks.lint.skill_permissions import …` in
  `tasks/build.py` is a hard circular import.** `tasks/lint/__init__.py:11`
  eagerly imports `vendor_shims`, which does `from tasks.build import
  _assert_magic_bytes` (`tasks/lint/vendor_shims.py:3`). Adding the import
  breaks `import tasks`, `tasks.build`, `tasks.lint` **and** `tasks.manifest` —
  every `mise run` entry point.
- **There are three SLSA blocks, not one.** The work item cites only
  `main.yml:423-425`.

Three further facts found during planning that constrain the design:

- **`tests/integration/tasks/test_github.py:252-258` patches
  `debug_archive_path` with a one-argument lambda** (`side_effect=lambda p: …`),
  and `_setup_release` also patches `RELEASE_MANIFEST` (`:271`) and
  `DISPATCHED_SUBBINARIES` (`:427`) as module constants. Any arity or default
  change to those three names ripples into that fixture, which every test in
  `TestUploadAndVerifyRelease` routes through — including the `assert
  len(uploads) == 22` pin (`:326`).
- **Eleven shipped skills declare bare `- Bash`** — `skills/config/migrate` plus
  all ten Linear and Jira integration skills. `has_bare_bash` is never itself a
  violation; it only *suppresses* the coverage check
  (`skill_permissions.py:167`). Those integration skills are plausible first
  consumers of 0170's token, so the guard's treatment of bare `Bash` is
  load-bearing rather than hypothetical.
- **`tasks/shared/__init__.py` is empty**, and the parsing core depends only on
  `re` and `fnmatch`, so a `tasks/shared/skill_parsing.py` leaf plus a
  `tasks/shared/dispatch_coherence.py` that imports it creates no cycle and no
  upward edge. Verified: all of `tasks`, `tasks.build`, `tasks.lint`,
  `tasks.manifest` and `tasks.shared.dispatch_coherence` import cleanly,
  individually and combined.

## Desired End State

- No literal `visualiser` remains anywhere on the dispatch-guard or
  release-staging path except in the two registry constants that legitimately
  name it (`DISPATCHED_SUBBINARIES`, `DEBUG_ARCHIVE_DIRS`), in the SLSA
  subject-path globs derived from the latter, and in the visualiser's own build
  tasks.
- The SKILL.md parsing primitives live in `tasks/shared/skill_parsing.py`, a
  leaf depending only on `re` and `fnmatch`, consumed by both
  `tasks/lint/skill_permissions.py` and the dispatch guard — so the two cannot
  drift to different notions of a bound token, and neither depends on the other.
- `validate_dispatch_coherence` lives in `tasks/shared/dispatch_coherence.py`,
  importing only from `tasks.shared.*`. It checks every non-exempt token in an
  injected collection in both directions, and rejects every registry shape that
  would make it vacuous or ship an undispatchable token — an empty collection, a
  stale exemption, an all-exempt collection, an exemption whose token is
  invoked, a reserved or built-in-shadowing token, and an invalid token charset
  — reporting each failure by cause and naming the offending SKILL.md.
- It runs on the release path (`tasks/manifest.py`, argument-free) **and** as
  `lint:dispatch-coherence:check` in `build-system:check` — the roll-up CI
  actually runs — so a skill author gets the failure on their PR rather than
  mid-release.
- The three release-stage builders plus the debug-archive producer take injected
  token collections under **one** idiom, on private helpers rather than on the
  publish `@task`, and their defaults are pinned by test.
- `.gitignore`'s archive rule covers the tree the archives are written into and
  `_ARTIFACT_MARKERS` backstops it, so a release artefact cannot reach the
  version-bump commit.
- `tasks/README.md` carries a `## Registering a dispatched sub-binary` checklist
  of thirteen points, whose registration-point names are pinned both by a
  literal-string test and by a source-existence assertion, so a rename in code
  fails a test rather than silently ageing the docs.

Verified by `mise run` exiting 0 end to end.

### Key Discoveries

- `collect_entries` (`tasks/manifest.py:69`) already has exactly the
  parameter-with-default shape the item wants, annotated `Iterable[str]` — the
  house precedent, and the annotation to copy.
- `test_both_absent_is_coherent` (`tests/unit/tasks/test_build.py:145-148`)
  asserts an empty collection **passes**. The anti-vacuity requirement is a
  direct behavioural reversal of a committed test. Deleting the enclosing class
  also strands `DispatchCoherenceError` at `test_build.py:18` — its only uses
  are `:136` and `:142` — which ruff `F401` catches (`tests/**` per-file-ignores
  cover `S`, `ANN`, `D`, `PLR2004`, `SLF001`, `PT`, `INP001`, not `F`).
- The other three `TestValidateDispatchCoherence` tests seed a SKILL.md body of
  ``"run `accelerator visualiser start`"`` — a backticked string the *new*
  invocation definition deliberately does not match. All must be rewritten, not
  re-parameterised.
- `.gitignore:54` is `bin/*.debug.tar.gz` — token-generic in the *token* but not
  in the *directory*: it matches only a directory literally named `bin`.
  `.gitignore:44`'s `bin/visualiser-*` sits in the launcher-cache block, whose
  comment (`:35-41`) says "and the sub-binary cache" — the launcher caches every
  fetched sub-binary as `bin/<token>-<version>-<sha256>` (`resolve/cache.rs:30`,
  `cache_root.rs:65`), so that entry is needed unconditionally, not only when
  something is staged there.
- Applying the invocation definition across the whole skills tree today yields
  exactly two launcher tokens: `config` (204 invocations, 45 skills) and
  `visualiser` (1 invocation, 1 skill). The invocation→registration direction
  therefore has a clean baseline.
- `is_root_help` (`cli/launcher/src/main.rs:102-108`) lists `Some("version" |
  "config" | "help")` — a hardcoded, *not* compile-enforced built-in list;
  clap's `Command` enum (`cli/launcher/src/launch/inbound/cli.rs:17-29`)
  declares only `Version` and `Config`, with `help` generated.
  `tests/unit/tasks/test_manifest_contract.py:51-74` is the house idiom for
  pinning a Python constant against a Rust literal.
- `tests/unit/tasks/test_workflows.py` already parses `main.yml` with `yaml`,
  and `test_attest_globs_include_the_launcher_binaries` (`:147-159`) already
  iterates every attest step asserting both `dist/release/accelerator-*` and
  `accelerator-visualiser-*` — so the SLSA work extends an existing test rather
  than adding a second, weaker one beside it. It also enumerates `Sign*` steps
  (`:127`), which is what makes "one attestation per signing step per job"
  cheap.
- `tests/conftest.py`'s `fake_repo_tree` writes no `skills/` tree, so the
  guard's tests need their own seeding helper.
  `tests/unit/tasks/test_skill_permissions.py:22-25` already has one (`_skill`),
  which is the shape to reuse rather than re-invent.
- There is no markdown formatter or linter anywhere in the repo, and
  `format:build-system:check` is `ruff format --check` over Python
  (`mise.toml:314-317`). The new README section's 80-column wrapping is a
  hand-checked convention.

## What We're NOT Doing

- **Not** collapsing registration to a single allowlist entry. Registration
  stays multi-step; this plan makes the steps documented and enforced.
- **Not** generalising `server_cross_compile` (`tasks/build.py:290-309`). It is
  the visualiser's own build task; checklist point 8 frames a new sub-binary's
  cross-compile staging as an author action.
- **Not** modifying `cli/launcher/tests/fixtures/manifest.example.json`. It is a
  shipped artefact; tests that need manifest data construct an in-test manifest.
- **Not** registering any fixture token in `DISPATCHED_SUBBINARIES`. Fixture
  tokens are test-only, passed through injected parameters, so the real signing,
  manifest and upload paths are untouched by the verification.
- **Not** adding a launcher built-in. Checklist point 10 documents the lockstep
  obligation, and Phase 2 pins the Python set against the Rust literal.
- **Not** making bare `Bash` a `skill_permissions` violation. Eleven skills rely
  on it today; the guard declines to treat it as a binding, but narrowing or
  allowlisting the eleven is a sibling task.
- **Not** narrowing `covered_by` to a `*`-only matcher. It models Claude Code's
  matcher and is untested against it; Phase 1 adds a matcher-contract table to
  make the model explicit, but changing the model is out of scope.

## Implementation Approach

Five phases, each landing green on top of its predecessors. The dependency graph
is **1→2, 3→4, {2,3,4}→5** — only Phases 1 and 3 are freely orderable, because
Phase 2's guard imports Phase 1's leaf, Phase 4's seams sit on Phase 3's final
shape, and Phase 5's checklist names symbols Phases 2 and 3 create *and* a
registry pin Phase 4 creates. "Independently mergeable" means each phase is a
complete, green change on top of what precedes it — not that the five can land
in any order.

Phase 1 extracts the parsing core to a dependency-free leaf and renames the
probe constant during the move — a pure refactor that removes the circular
import at its cause rather than routing around it, and makes `tasks/shared/` a
legitimate home for the guard. Phase 2 delivers the guard on top of it and wires
it into both the release path and a lint leaf CI actually runs. Phase 3 removes
the last visualiser-shaped release stage. Phase 4 unifies the injection idiom
across the builders on top of Phase 3's final shape, so `_release_uploads` is
edited once into its end state rather than twice. Phase 5 documents the whole
surface.

Test-driven throughout: each phase writes its failing tests first, against
fixture tokens injected through the new parameters.

### Decisions locked during planning

These resolve the research's seven open questions, the work item's one, and the
design calls raised by plan review 1; none remain open.

| Decision | Choice | Why |
|---|---|---|
| Circular-import remedy | Parsing core extracted to `tasks/shared/skill_parsing.py` (Phase 1); guard at `tasks/shared/dispatch_coherence.py` importing it | Fixes the cause — a pure parser trapped behind `invoke` and `vendor_shims` — rather than relocating the consumer. Both `tasks/shared/` modules then import only `tasks.shared.*`, so the leaf layer stays a leaf and no import-direction footgun remains to pin. |
| Permission check | Probe with the **actual invoked command**, plus a structural assertion that the rule's token segment equals the token exactly | The synthetic probe is wrong in both directions (see the table above). Matching the real command is byte-for-byte what `skill_permissions` enforces, so the guard cannot disagree with the lint rule; the segment check rejects `v*`, `[a-y]*` and `?isualiser`, which the `BARE_LAUNCHER` probe cannot. |
| `BARE_LAUNCHER` probe | **Removed** from `_authorises`; retained in the skill-level veto, where it is load-bearing | Inside `_authorises` the conjunct was dead: segment equality forces the rule's segment to be the literal token, and the veto already guarantees no rule covers `BARE_LAUNCHER` whenever `_authorises` runs. In the veto it earns its place — verified that it catches `${CLAUDE_PLUGIN_ROOT}/bin/*`, `${CLAUDE_PLUGIN_ROOT}/*` and `…/accelerator *` (all `True`), which the charset check below cannot see because they yield no token segment at all. |
| Over-broad skills | Vetoed at **skill** level by a two-part test: bare `Bash`, or any rule that covers `BARE_LAUNCHER`, or any rule whose token segment is not a bare token | The two parts cover disjoint shapes. Verified: the sentinel catches globs at or above the binary segment but returns `False` for `…/accelerator [a-y]*` and `…/accelerator v*`, so without the charset half a skill carrying a correctly scoped rule *alongside* `[a-y]*` would be certified cleanly bound while pre-authorising every token not starting with `z`. Skill-level rather than per-rule, so a skill carrying both shapes cannot witness a narrow binding. |
| Exemption bounds | Guard raises on `set(exempt) - set(tokens)`, on `set(tokens) <= set(exempt)`, and on an exempt token that **is** invoked by a skill | The first two close cardinality vacuity. The third enforces the exemption's stated bar — "invoked only by hooks or another binary, never from a SKILL.md" — which cardinality alone does not: without it a token whose consuming skill exists but carries only bare `Bash` or an ancestor glob can be exempted, and the ten bare-`Bash` Linear/Jira skills make that the path of least resistance for 0170. |
| Reserved and colliding tokens | Guard raises on `verify`/`launcher`, on any token in `BUILTIN_SUBCOMMANDS`, and on a token outside `^[a-z][a-z0-9-]*$` | Prose that is not mechanically checked ages silently — this plan's own thesis. `verify` is the sharp case: `subbinary_asset_path("verify", …)` and `cli_binary_path("accelerator-verify", …)` collide in `dist/release/`, so registering it would sign the vendored verify shim and advertise it in the manifest. A built-in-shadowed token would be signed and published but never dispatchable. |
| Built-in set | `version`, `config`, **`help`**, pinned against the clap `Command` enum with `is_root_help` as a secondary check | `is_root_help` is a help-*routing* heuristic; the dispatch authority is `Command` (`cli/launcher/src/launch/inbound/cli.rs:17-29`), which declares `Version`, `Config` and `External` only, with `help` clap-generated. Pinning the heuristic alone would adopt a name added there into the allowlist and silently exempt invocations the launcher routes to `External`. |
| Guard placement | The argument-free release call **plus** `lint:dispatch-coherence:check` in `build-system:check` *and* `lint:check` | The scan is pure text over committed files; failing after four cross-compiles and a use of the signing secret is the wrong place to learn. Verified: `check` does **not** depend on `lint:check` and no CI job runs `lint:check`, so `lint:check` alone would reproduce `lint:skill-permissions:check`'s CI blind spot. `build-system:check` is what CI runs and already carries the non-component `lint:workflows:check`. Dual placement mirrors `_CLI_CHECK_GATES`' documented reasoning. |
| Debug-archive hardwiring | Absorbed here (Phase 3); the `.gitignore` rule is **fixed** and the leak guard extended, which is what makes a `bin/` constraint meaningful | Directed; overrides the item's "local to the token loop" boundary rule. Verified in a scratch repo that `bin/*.debug.tar.gz` is root-anchored and does **not** match `skills/visualisation/visualise/bin/` — so today's archives are untracked-and-unignored and a `bin/`-suffix constraint alone would encode a false invariant. Phase 3 changes the pattern to `**/bin/*.debug.tar.gz`, adds `.debug.tar.gz` to `_ARTIFACT_MARKERS`, and corrects `.gitignore:28`'s now-stale "are tracked" comment. |
| Docs surface | Thirteenth checklist point | Directed; amends the item's "exactly ten numbered items" criterion to thirteen — an eleventh for user-facing docs, a twelfth for `DEBUG_ARCHIVE_DIRS` (which Phase 3 creates as a registration point), and a thirteenth for `cli/deny.toml`, the one registration-adjacent gate CI already enforces on every PR. |
| Checklist enforcement | Literal strings **and** source-existence assertions | A test that reads only the README freezes doc text: a rename in code leaves it green, while correcting the README makes it fail. The pair is what makes a rename fail a test. |
| Imperative-action predicate | Kept, with the closed verb set named in this plan and all thirteen items drafted against it | The predicate is not decidable in general — a review-1 accepted major — and a verb whitelist coupled to prose is genuinely brittle. Retained because an acceptance criterion requires it; the mitigation is that the set is explicit in both the plan and the test, so section and test are drafted against each other rather than discovered to disagree. |
| SLSA blocks | All three named; the existing test extended to assert one attestation per signing step per job, `subject-path` set equality, and coverage derived from **`_release_uploads()`** | Symmetry is not coverage, and `> 1` does not pin the count. Deriving from the actual publish set rather than from `DEBUG_ARCHIVE_DIRS` is what catches the gap verified today: `manifest.json` and `manifest.minisig` are uploaded but matched by no `subject-path` glob, so the signed manifest — the highest-value artefact in the distribution — currently ships unattested. Phase 3 adds both to all three blocks. |
| Heading level | `##` | The acceptance criterion's literal. |
| `…/bin/accelerator` with no token | Ignored by the guard | No token to check. `skill_permissions` already flags bare-launcher *rules*. |
| Flag-shaped first argument | Ignored by the guard | `--version` is not a token, and "needs an entry in DISPATCHED_SUBBINARIES" would be wrong to follow. `is_root_help` already treats bare flags as root help. |
| Injection idiom | Direct defaults of the real constant on the **private helpers**, annotated `Iterable[str]` per `collect_entries`; every `@task` stays argument-free | Required by the guard's identity-pinning criterion, and one rule is what five sibling stories can follow. `upload_and_verify_release` and `create_debug_archives` are both invoke `@task`s, so a parameter becomes an operator-facing CLI flag on a release-path leaf — and a `Mapping[str, Path]` default is not expressible on a command line at all. Confined to token collections: `RELEASE_MANIFEST`/`RELEASE_MANIFEST_SIG` stay module globals read at call time, and `test_github.py:271`/`:272`/`:427`'s module patches stay — patching is the correct mechanism where no seam exists by design. |
| Manifest/upload agreement | `_publish` asserts the on-disk `manifest.json`'s `binaries` set equals the dispatched tokens, immediately after `_assert_no_leaked_artifacts` | Verified this is reachable, unlike a check inside `emit_manifest`: at the only `emit_manifest` call site (`tasks/release.py:_sign`) `entries` comes from `collect_entries()` and both default to `DISPATCHED_SUBBINARIES`, so the two operands have one source and the comparison is a tautology. In `_publish` the manifest is read back from disk, and `release:finalise` is separately invocable — so a `dist/release/` left from an earlier cut is a real divergence, caught before `commit_version`/`tag_version`/`push`/`create_release` rather than after the tag is pushed. |
| Test homes | Guard in `tests/unit/tasks/shared/test_dispatch_coherence.py`, parsing in `tests/unit/tasks/shared/test_skill_parsing.py`, path helpers in `tests/unit/tasks/shared/test_paths.py` (with `TestCliPathHelpers` moved there), README in `tests/unit/tasks/test_registration_docs.py`, signing in `tests/unit/tasks/test_signing.py`, github builders in `tests/integration/tasks/test_github.py`, the `emit_manifest` spy in `tests/unit/tasks/test_manifest.py` | Honours the `tests/unit/tasks/shared/` mirror the repo maintains one-to-one, keeps each module's coverage in one file, and gives the cross-cutting docs guard its own file per the `test_mise.py` / `test_workflows.py` precedent. |

---

## Phase 1: Extract the SKILL.md parsing into a dependency-free leaf

### Overview

Move the pure parsing primitives out of `tasks/lint/skill_permissions.py` into a
new `tasks/shared/skill_parsing.py` that imports only `re` and `fnmatch`, and
rename `_BARE_LAUNCHER` to `BARE_LAUNCHER` as part of the move. Pure refactor,
no behaviour change — but it removes the circular import at its cause, so the
guard can live in `tasks/shared/` without any module there importing upward.

### Changes Required

#### 1. The parsing leaf

**File**: `tasks/shared/skill_parsing.py` (new)
**Changes**: Move, unchanged except for the constant rename, everything in
`skill_permissions.py` that depends only on `re` and `fnmatch`:

- functions `_frontmatter_lines`, `frontmatter_bash_rules`, `has_bare_bash`,
  `frontmatter_name`, `preprocessor_commands`, `is_plugin_invocation`,
  `covered_by`, `has_metacharacter`
- constants `_BASH_RULE`, `_PREPROCESSOR`, `_BARE_BASH_LINE`, `_NAME_LINE`,
  `_METACHARACTERS`, `PLUGIN_PREFIX`, and `BARE_LAUNCHER` (renamed in the move)

Plus one **new** primitive, because it is parsing and both consumers need the
same answer — the launcher path and the token extractor:

```python
LAUNCHER = f"{PLUGIN_PREFIX}bin/accelerator"
# A launcher command naming no subcommand — any rule matching it is too broad.
# The sentinel argument is load-bearing: `covered_by` appends `*` to a rule that
# lacks one, so a bare `{LAUNCHER}` probe matches even a correctly scoped rule.
BARE_LAUNCHER = f"{LAUNCHER} zz-external-subcommand-zz"


def launcher_token(text: str) -> str:
    """The subcommand token in a launcher command or Bash rule, else empty.

    Applied to both, so a rule's token segment is extracted by exactly the code
    that extracts an invocation's — which is what lets the two be compared for
    equality rather than by glob.
    """
    if not text.startswith(LAUNCHER):
        return ""
    tail = text[len(LAUNCHER) :]
    # The prefix match must be followed by a separator: a sibling binary whose
    # name continues `accelerator` would otherwise yield a token spliced out of
    # the middle of its filename.
    if not tail.startswith(" "):
        return ""
    parts = tail.split()
    if not parts or parts[0].startswith("-"):
        return ""
    return parts[0]
```

Deriving `BARE_LAUNCHER` from `LAUNCHER` is the point: previously the constant
hardcoded `${CLAUDE_PLUGIN_ROOT}/bin/accelerator …` while the guard
independently rebuilt the same prefix from `PLUGIN_PREFIX`, so the path existed
twice by two constructions — the desynchronisation the constant's own comment
warns about, and load-bearing because segment equality and the bare-launcher
probe must be talking about the same launcher.

The module docstring states the one non-obvious fact: `covered_by` models Claude
Code's `Bash(...)` matcher, and both a lint rule and a release gate now depend
on that model agreeing with the real one.

What **stays** in `skill_permissions.py`: `EXPECTED_INJECTION_SKILLS`, the
config-census markers (`_CONFIG_MARKER`, `_CONTEXT_SKILL`, `_CONTEXT_ANY`,
`_INSTRUCTIONS`, `_NAME_TOKEN`), `_name_after`, `_command_violations`,
`_check_skill`, `violations` and `@task check`. Those are lint policy, not
parsing, and they are what depend on `invoke` and `tasks.shared.sources`.

#### 2. The lint module re-points at the leaf

**File**: `tasks/lint/skill_permissions.py`
**Changes**: Replace the moved definitions with `from tasks.shared.skill_parsing
import (...)`, importing **only the names the module still uses** —
`frontmatter_bash_rules`, `has_bare_bash`, `frontmatter_name`,
`preprocessor_commands`, `is_plugin_invocation`, `covered_by`,
`has_metacharacter`, `BARE_LAUNCHER` — and update the sole `_BARE_LAUNCHER`
caller at `:187`. `PLUGIN_PREFIX` is **not** re-imported: its only in-module use
was inside `is_plugin_invocation`, which moves, so importing it purely to
re-export would be an `F401` failure under `select = ["ALL"]`.

This module is deliberately **not** a re-export façade. That means one consumer
moves with it, which is the honest cost of a single home for the parsing
contract.

**File**: `tests/integration/support/skill_corpus.py`
**Changes**: Re-point `:21-26`'s `from tasks.lint.skill_permissions import
(PLUGIN_PREFIX, frontmatter_name, is_plugin_invocation, preprocessor_commands)`
at `tasks.shared.skill_parsing`. This is the third consumer of the parsing
surface — it backs the `!`-site conformance corpus — and `PLUGIN_PREFIX` is the
name that forces the decision.

`tests/unit/tasks/test_skill_permissions.py:74-79` asserts on the message
substring `"without a subcommand"` and never imports the constant, so it needs
no change.

#### 3. Tests

**File**: `tests/unit/tasks/shared/test_skill_parsing.py` (new)
**Changes**: Verified: **no direct test of any parsing primitive exists today**
— all ten tests in `test_skill_permissions.py` drive `violations(tmp_path)` over
a synthetic skill tree, and `covered_by`, `frontmatter_bash_rules`,
`has_bare_bash`, `frontmatter_name`, `preprocessor_commands`,
`is_plugin_invocation` and `has_metacharacter` have none. So this file is
authored, not moved: seven functions are being promoted to a contract two
independent guards depend on, and they would otherwise arrive with no coverage.

Author, at minimum:

- **`covered_by` matcher-contract table** — a parametrised test over explicit
  `(command, rule, expected)` triples: an exact rule; a rule with a trailing
  `*`; a rule without one (silently widened to match trailing arguments); a rule
  whose `*` spans `/`; the `?` / `[seq]` / `[!seq]` classes `fnmatch` honours;
  and the **path-alias and quoting forms** — `…/bin/../bin/accelerator *`,
  `…/./bin/accelerator *`, `…//bin/accelerator *`, `"…/bin/accelerator" *` —
  each recorded with its actual outcome. The aliases are not matched by
  `covered_by(BARE_LAUNCHER, rule)` and yield no token, so today they evade both
  the lint rule's over-broad-rule check and the guard in both directions. The
  table's job is to make that legible now two guards depend on it; closing it is
  out of scope (see *What We're NOT Doing*).
- **`launcher_token`** — the visualiser's real invocation; a rule at token level
  with and without a trailing `*`; a wildcarded segment (`v*`, `[a-y]*`); a
  flag-first argument; `bin/accelerator-verify-<platform>` and a
  `Bash(…/accelerator-verify *)` rule (both must yield `""`); and a name that
  continues `accelerator` without a separator, which is the case the space check
  uniquely closes.
- **frontmatter parsing** — a missing and an unterminated `---` fence; multiple
  `Bash(...)` rules on one line and across lines; the bare-`Bash` line variants
  `_BARE_BASH_LINE` accepts; quoted vs unquoted `name:`.
- **`preprocessor_commands`** — multi-command extraction in document order.
- **`has_metacharacter`** — one case per entry in `_METACHARACTERS`.

**File**: `tests/unit/tasks/test_skill_permissions.py`
**Changes**: Add a source-scan test asserting the literal `_BARE_LAUNCHER`
appears nowhere under `tasks/`, so a retained alias cannot satisfy the rename.
Follows the `test_bootstrap_coverage.py:54-74` regex-over-`read_text` idiom.

**File**: `tests/unit/tasks/shared/test_skill_parsing.py`
**Changes**: Add a source scan asserting `tasks/shared/skill_parsing.py` imports
nothing from `invoke` or from any `tasks.` module — the invariant that keeps it
a leaf, and the reason this phase exists. Resolve the repo root with
`repo_root()` from `tasks.shared.sources`, not a depth-coupled
`Path(__file__).parents[N]`: the `tests/unit/tasks/*.py` idiom is `parents[3]`,
which is off by one a directory deeper.

### Success Criteria

#### Automated Verification

- [ ] Parsing suite passes: `uv run pytest
      tests/unit/tasks/shared/test_skill_parsing.py -v`
- [ ] Rename guard and lint suite pass: `uv run pytest
      tests/unit/tasks/test_skill_permissions.py -v`
- [ ] Lint task still green: `mise run lint:skill-permissions:check`
- [ ] Every entry point still imports: `uv run python -c "import tasks,
      tasks.build, tasks.lint, tasks.manifest"`
- [ ] Build-system checks pass: `mise run build-system:check`
- [ ] Full unit suite passes: `mise run test:unit:tasks`

#### Manual Verification

- [ ] `rg '_BARE_LAUNCHER' tasks/` returns nothing.
- [ ] `rg 'import' tasks/shared/skill_parsing.py` shows only `fnmatch` and `re`.
- [ ] `git diff` on the moved functions is a pure move — no logic change beyond
      the constant rename.

---

## Phase 2: The generalised guard

### Overview

Replace the visualiser-hardcoded guard with a token-generic one in a new
`tasks/shared/dispatch_coherence.py`, add `SKILL_EXEMPT_SUBBINARIES`, wire it
into both the release path and a lint leaf CI actually runs, and cover every
pass/fail path against fixture tokens.

### Changes Required

#### 1. The exemption registry

**File**: `tasks/shared/paths.py`
**Changes**: Add a sibling constant to `DISPATCHED_SUBBINARIES` (`:25`), empty
when this lands.

```python
# Tokens whose only consumer is a hook or another binary, never a SKILL.md.
SKILL_EXEMPT_SUBBINARIES: tuple[str, ...] = ()
```

#### 2. The guard

**File**: `tasks/shared/dispatch_coherence.py` (new)
**Changes**: The whole guard plus its helpers, importing only from
`tasks.shared.*` — Phase 1 is what makes that possible. No literal `visualiser`
appears in this file, which is what makes the source scan a whole-file scan. The
launcher path and the token extractor live in the parsing leaf, not here, so the
prefix has one spelling.

```python
import re
from collections.abc import Iterable
from pathlib import Path

from tasks.shared.errors import DispatchCoherenceError
from tasks.shared.paths import (
    DISPATCHED_SUBBINARIES,
    REPO_ROOT,
    SKILL_EXEMPT_SUBBINARIES,
)
from tasks.shared.skill_parsing import (
    BARE_LAUNCHER,
    LAUNCHER,
    covered_by,
    frontmatter_bash_rules,
    has_bare_bash,
    has_metacharacter,
    is_plugin_invocation,
    launcher_token,
    preprocessor_commands,
)

# Must equal the launcher's built-in set; a test pins it against the clap
# `Command` enum, which is not compile-enforced from this side.
BUILTIN_SUBCOMMANDS = frozenset({"version", "config", "help"})
# Staged-but-never-dispatched binaries whose asset name a token would collide
# with, plus `launcher`. Derived by test from _CLI_RELEASE_BINARIES minus the
# dispatched set: a third staged binary cannot silently become registrable, and
# a token whose own binary is staged there stays legal.
RESERVED_TOKENS = frozenset({"verify", "launcher"})
_TOKEN = re.compile(r"^[a-z][a-z0-9-]*$")
```

The rule matcher applies **two** conditions to a single rule. Requiring the
extracted segment to equal the token exactly is what rejects `v*`, `[a-y]*` and
`?isualiser`; matching the real `command` rather than a synthetic probe is what
makes this agree with `skill_permissions`' coverage check by construction:

```python
def _authorises(rules: list[str], *, token: str, command: str) -> bool:
    return any(
        launcher_token(rule) == token and covered_by(command, rule)
        for rule in rules
    )
```

`token` and `command` are keyword-only: they are adjacent and same-typed, and a
transposed positional call would type-check, pass every negative test, and
silently make the guard vacuous on the release path.

There is deliberately no `not covered_by(BARE_LAUNCHER, rule)` conjunct here. It
would be dead code: segment equality forces the rule's segment to be the literal
token, and the skill-level veto below already guarantees that no rule in `rules`
covers `BARE_LAUNCHER` whenever `_authorises` is reached. `BARE_LAUNCHER` is
used in the veto instead, where it catches shapes the charset check cannot.

**The over-broad-skill veto** is two-part, because the two halves cover disjoint
shapes — verified against `covered_by`:

| Rule | covers `BARE_LAUNCHER` | token segment |
|---|---|---|
| `${CLAUDE_PLUGIN_ROOT}/*` | `True` | none |
| `${CLAUDE_PLUGIN_ROOT}/bin/*` | `True` | none |
| `…/accelerator *` | `True` | `*` |
| `…/accelerator [a-y]*` | **`False`** | `[a-y]*` |
| `…/accelerator v*` | **`False`** | `v*` |
| `…/accelerator visualiser *` | `False` | `visualiser` |

The sentinel catches everything at or above the binary segment; the charset
check catches a wildcarded token segment. Either alone leaves a hole — without
the charset half, a skill carrying a correctly scoped rule *alongside*
`Bash(…/accelerator [a-y]*)` would be certified cleanly bound while
pre-authorising every token not starting with `z`.

```python
def _is_over_broad(text: str, rules: list[str]) -> bool:
    return (
        has_bare_bash(text)
        or any(covered_by(BARE_LAUNCHER, rule) for rule in rules)
        or any(
            segment and not _TOKEN.match(segment)
            for segment in (launcher_token(rule) for rule in rules)
        )
    )
```

The scan returns the two directions as two separate values — a set of bound
tokens and a map from invoked token to the first SKILL.md that invokes it, which
is what lets each error name the offending file:

```python
def _bindings(root: Path) -> tuple[set[str], dict[str, str]]:
    bound: set[str] = set()
    invoked: dict[str, str] = {}
    for path in sorted((root / "skills").rglob("SKILL.md")):
        text = path.read_text()
        rules = frontmatter_bash_rules(text)
        over_broad = _is_over_broad(text, rules)
        rel = path.relative_to(root).as_posix()
        for command in preprocessor_commands(text):
            for token in _every_token(command):
                invoked.setdefault(token, rel)
            if not is_plugin_invocation(command):
                continue
            # `skill_permissions` refuses to coverage-check a chained command,
            # so binding on one would let the two guards disagree.
            if has_metacharacter(command):
                continue
            token = launcher_token(command)
            if token and not over_broad and _authorises(
                rules, token=token, command=command
            ):
                bound.add(token)
    return bound, invoked
```

The two directions read the command differently, on purpose. **Binding** is
strict: prefix-anchored, metacharacter-free, one token. The
**invocation→registration** direction is deliberately permissive, because it is
the fail-*closed* half — a token it cannot see is a token that ships
unregistered. `_every_token` therefore finds every occurrence of the launcher
path anywhere in the command text, so `cd . &&
${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs status` still registers `vcs` even
though neither `is_plugin_invocation` nor the strict tokeniser would:

```python
def _every_token(command: str) -> set[str]:
    tokens: set[str] = set()
    for index in _launcher_occurrences(command):
        token = launcher_token(command[index:])
        if token:
            tokens.add(token)
    return tokens
```

`_launcher_occurrences` yields each index at which `LAUNCHER` appears (a plain
`str.find` loop). Making this half permissive is what closes the hole a
prefix-anchored scan leaves in the exemption bar too: an exempt token invoked
mid-chain is now caught.

**The registry validation**, split out so the pure core below reads as two
directions and nothing else:

```python
def _registry_problems(
    tokens: tuple[str, ...], exempt: tuple[str, ...]
) -> list[str]:
    if not tokens:
        return [
            "no dispatched sub-binaries resolved — DISPATCHED_SUBBINARIES was "
            "lost rather than deliberately emptied"
        ]
    problems = [
        f"{token}: not a valid token — must match {_TOKEN.pattern}, because it "
        "derives ACCELERATOR_<TOKEN>_BIN, which the launcher refuses to build "
        "from a name outside that set"
        for token in tokens
        if not _TOKEN.match(token)
    ]
    problems.extend(
        f"{token}: reserved — its staged asset name or default crate path "
        "collides with the launcher's or the verify shim's"
        for token in sorted(set(tokens) & RESERVED_TOKENS)
    )
    problems.extend(
        f"{token}: shadows a launcher built-in, so it would be signed and "
        "listed in the manifest but never dispatched"
        for token in sorted(set(tokens) & BUILTIN_SUBCOMMANDS)
    )
    problems.extend(
        f"{token}: exempt but not dispatched — either the token was dropped "
        "from DISPATCHED_SUBBINARIES or the exemption is stale"
        for token in sorted(set(exempt) - set(tokens))
    )
    if set(tokens) <= set(exempt):
        problems.append(
            "every dispatched sub-binary is exempt — this guard would check "
            "nothing; an exemption is for a token consumed only by a hook or "
            "another binary, not a way to silence a failure"
        )
    return problems
```

**The pure core** follows the house `violations(root, …) -> list[str]` shape
(`skill_permissions.violations`, `claude_coupling.violations`) with a
**required** root, so a rootless test can never silently scan the real
`skills/**/SKILL.md` — the reasoning `cli_member_manifests` already records in
`tasks/shared/paths.py` for its own required argument:

```python
def violations(
    root: Path,
    *,
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
    exempt: Iterable[str] = SKILL_EXEMPT_SUBBINARIES,
) -> list[str]:
    """Every dispatch-coherence problem, in both directions.

    Registry problems short-circuit: a malformed constant makes the skills scan
    meaningless, so they are reported alone.

    A token is bound when at least one SKILL.md invokes `accelerator <token>`
    through the `!` preprocessor and carries a `Bash(...)` rule whose subcommand
    segment is exactly that token and which covers the invocation, in a skill
    that declares no bare `Bash` tool, no rule authorising the bare launcher and
    no rule with a wildcarded token segment.

    An exemption declares that no SKILL.md invokes the token; one that is
    invoked, or that names an undispatched token, or that covers every token, is
    itself a problem. So an exemption requires at least one non-exempt token.
    """
    names, exemptions = tuple(tokens), tuple(exempt)
    problems = _registry_problems(names, exemptions)
    if problems:
        return problems

    bound, invoked = _bindings(root)
    for token in names:
        # The exemption check precedes the `bound` short-circuit: an exemption
        # asserts that no SKILL.md invokes the token, so one that gained a real
        # binding must surface as stale rather than pass.
        if token in exemptions:
            if token in invoked:
                problems.append(
                    f"{token}: exempt but {invoked[token]} invokes "
                    f"`accelerator {token}` — an exemption is for a token no "
                    "SKILL.md invokes; drop the exemption"
                )
            continue
        if token in bound:
            continue
        if token not in invoked:
            problems.append(
                f"{token}: no skill invokes `accelerator {token}` through the "
                "`!` preprocessor — add a consuming skill, or an entry in "
                "SKILL_EXEMPT_SUBBINARIES if its only consumer is a hook"
            )
        else:
            problems.append(
                f"{token}: {invoked[token]} invokes `accelerator {token}` but "
                "declares no Bash(...) rule naming that subcommand — a bare "
                "`Bash` tool, a rule authorising the bare launcher, or a rule "
                "with a wildcarded token segment disqualifies the skill"
            )
    problems.extend(
        f"{token}: {invoked[token]} invokes `accelerator {token}`, which is "
        "neither dispatched nor a launcher built-in — rename the subcommand if "
        "it is reserved or invalid, otherwise add it to DISPATCHED_SUBBINARIES"
        for token in sorted(invoked)
        if token not in names and token not in BUILTIN_SUBCOMMANDS
    )
    return problems


def validate_dispatch_coherence(
    repo_root: Path | None = None,
    *,
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
    exempt: Iterable[str] = SKILL_EXEMPT_SUBBINARIES,
) -> None:
    """Raise if any dispatch-coherence problem exists. See violations()."""
    problems = violations(
        repo_root or REPO_ROOT, tokens=tokens, exempt=exempt
    )
    if problems:
        raise DispatchCoherenceError(
            "dispatch coherence found problem(s):\n  " + "\n  ".join(problems)
        )
```

The defaults live on **both** entry points, because the release path calls
`validate_dispatch_coherence()` and the lint leaf calls
`violations(repo_root())` — so the identity pin covers both signatures.
Iterating `names` in registry order is what satisfies the "the error names the
second token" criterion: the *filter*, not a sort, is what makes a two-token
fixture report only the unbound one, and registry order is the order an author
recognises.

#### 3. Removals and re-pointing

**File**: `tasks/build.py`
**Changes**: Delete `_VISUALISE_SKILL_RELATIVE` (`:35`) and
`validate_dispatch_coherence` (`:189-208`). Drop `DispatchCoherenceError` and
`DISPATCHED_SUBBINARIES` from the imports if nothing else in the file uses them.

**File**: `tasks/manifest.py`
**Changes**: Import `validate_dispatch_coherence` from
`tasks.shared.dispatch_coherence` instead of `tasks.build` (`:7-10`), and move
the argument-free call from `:138` to the **first** statement of
`emit_manifest`. The guard reads nothing from the manifest, so running it before
`atomic_write_text` avoids leaving a freshly written unsigned `manifest.json`
beside a stale `manifest.minisig` on failure. `validate_version_coherence` still
runs after the write, so that torn state stays reachable via a version mismatch;
it fails closed downstream at `_reverify_via_shim`'s
`manifest.json`/`manifest.minisig` check, before `--draft=false`.

This placement buys **no** fail-fast benefit, and the plan does not claim one:
`tasks/release.py:_sign` calls `signing.sign_staged_binaries(key)` *before*
`emit_manifest`, so by the time the guard runs, four cross-compiles are done and
the signing secret has already been used on eight binaries. The argument-free
call stays here only because an acceptance criterion pins it to manifest
generation. The gate that fires early is `lint:dispatch-coherence:check` in
`build-system:check`, which is why this phase adds it.

#### 4. The lint task

Verified: `check` depends on
`frontend/server/cli/deny/pup/build-system/scripts:check` and **not** on
`lint:check`, and no CI job runs `lint:check` or bare `check`. So `lint:check`
alone would reproduce `lint:skill-permissions:check`'s blind spot — green in CI
however badly the invariant is broken. The guard therefore rides
`build-system:check`, which CI runs (`main.yml:181`, a `needs` of the
`prerelease` job) and which already carries the non-component
`lint:workflows:check`, **and** `lint:check`, the only path a bare `mise run`
reaches. That dual placement is the reasoning `_CLI_CHECK_GATES` already
records.

Registering an invoke lint leaf is a **four**-file surface:

**File**: `tasks/lint/dispatch_coherence.py` (new)
**Changes**: A `@task check` calling `violations(repo_root(),
tokens=DISPATCHED_SUBBINARIES, exempt=SKILL_EXEMPT_SUBBINARIES)` and raising
`invoke.Exit` with the house `"\n  ".join(...)` formatting, following
`lint/skill_permissions.py:239-248`. Named for the invariant like every sibling
leaf, and matching the shared module's own name so leaf, module, mise leaf and
test file are one string.

**File**: `tasks/lint/__init__.py`
**Changes**: Add `dispatch_coherence` to the import tuple and to `__all__`.

**File**: `tasks/__init__.py`
**Changes**: Add
`ns_lint.add_collection(Collection.from_module(lint.dispatch_coherence))` beside
the existing eleven (`:84-116`), with the trailing `#
lint.dispatch-coherence.check` comment every sibling entry carries. Without this
the mise leaf resolves to nothing.

**File**: `mise.toml`
**Changes**: Add the `lint:dispatch-coherence:check` leaf with a `description`
and `depends = ["deps:install:python"]`, matching `lint:skill-permissions:check`
(`:407-410`), declared beside `lint:workflows:check` (`:339-345`) since it is
`build-system:check` it rides. Add it to both `build-system:check` (`:349`) and
`lint:check` (`:466`), and amend `build-system:check`'s description — currently
"Run all Python format, lint, and type checks plus workflow lint (ruff + pyrefly
+ actionlint)" — to name the new member, as it already names actionlint.

**File**: `tasks/README.md`
**Changes**: Update the `build-system:check` row in the per-component table
(`:18`) so the roll-up's stated scope matches what it folds, and add a sentence
to the conventions section recording the dual placement, mirroring the
`cli:check` paragraph at `:32-38`.

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: Add `_BUILD_SYSTEM_CHECK_GATES = ["lint:dispatch-coherence:check"]`
asserted present in `build-system:check.depends` via a new
`test_gate_wired_into_build_system_check`, and widen the existing
`test_gate_wired_into_lint_check` parametrisation to `_CLI_CHECK_GATES +
_BUILD_SYSTEM_CHECK_GATES`. Naming it for the roll-up it guards follows
`_CHECK_GATES`/`_CLI_CHECK_GATES`; it cannot join `_CLI_CHECK_GATES`, which
would additionally force it into `cli:check` — wrong for a skills-tree guard.

**File**: `tests/unit/tasks/test_dispatch_coherence.py` (new)
**Changes**: Prove the leaf *fails*, not just that it passes. Following
`tests/unit/tasks/test_claude_coupling.py:144`/`:151` and
`tests/unit/tasks/test_lint.py:43`/`:50`: patch the module's `violations` to
return a one-element list and assert `pytest.raises(Exit, match=...)` carries
that text; patch it to return `[]` and assert it does not raise. Without this an
inverted condition or a `print` in place of the `raise` leaves the PR-time gate
green forever.

#### 5. Tests

**File**: `tests/unit/tasks/shared/test_dispatch_coherence.py` (new)
**Changes**: A seeding helper writing SKILL.md files with real frontmatter, plus
the full matrix. Each case injects fixture tokens; none touches
`DISPATCHED_SUBBINARIES`. The helper follows `test_skill_permissions.py:22-25`'s
`_skill` shape, extended with the two cases that shape cannot express:

```python
def _skill(root, rel, *, rules=(), commands=(), prose="", bare_bash=False):
    path = root / "skills" / rel / "SKILL.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    allowed = [f"Bash({rule})" for rule in rules]
    if bare_bash:
        allowed.insert(0, "Bash")
    tools = "".join(f"\n  - {entry}" for entry in allowed)
    body = "\n".join([*(f"!`{command}`" for command in commands), prose])
    path.write_text(f"---\nname: {rel}\nallowed-tools:{tools}\n---\n{body}\n")
```

Every raising case asserts a **discriminating substring** where one exists;
where several cases share a message, the row states which mutation it kills
instead, so the reason each row exists survives into the test docstrings. Every
passing case states what else the tree must contain so the pass is not
accidental.

| Case | Expectation |
|---|---|
| Fixture token bound by a scoped rule + real invocation | passes |
| Rule scoped *tighter* than the token (`… <token> start`, invocation `… <token> start`) | passes |
| Rule scoped to a flag glob (`… <token> --owner-pid *`) covering the invocation | passes |
| Rule `… <token> start`, invocation `… <token> status` | raises — kills a dropped `covered_by(command, rule)` conjunct |
| Two rules `… <token> start` and `… <tok>*`, invocation `… <token> status` | raises — kills splitting the conjunction across rules |
| Fixture token with no invoking skill | raises, `"no skill invokes"` |
| Two-token collection, second unbound, first bound | raises, names the **second** only |
| Empty collection | raises, `"lost rather than deliberately emptied"` |
| Consuming skill carries only `Bash(…/accelerator *)` | raises, names the SKILL.md — kills the sentinel half of the veto |
| Consuming skill carries only `Bash(${CLAUDE_PLUGIN_ROOT}/bin/*)` | raises — sentinel half, above the binary segment |
| Consuming skill carries a scoped rule **and** `Bash(…/accelerator *)` | raises — skill-level, not per-rule |
| Consuming skill carries a scoped rule **and** `Bash(…/accelerator [a-y]*)` | raises — kills the charset half of the veto |
| Consuming skill carries only `Bash(…/accelerator <tok>*)` | raises — wildcarded segment |
| Consuming skill carries only a rule for a *different* subcommand (`… config *`) | raises — segment equality |
| Skill mentions the token only in prose **and** in a backticked reference | raises, `"no skill invokes"` |
| Skill invokes a *different* token that is itself registered and bound | raises for the target token; message excludes `"neither dispatched"` |
| Two skills invoke the token; one scoped rule, one ancestor glob | passes |
| Skill invokes `accelerator zz-unregistered-zz` | raises, `"neither dispatched"`, names the SKILL.md |
| Skill invokes `accelerator version` / `config` / `help`, no collection entry | passes (parametrised over all three, each with a separately bound fixture token present) |
| Skill invokes `accelerator --version` | passes — flag, not a token |
| Skill invokes `…/bin/accelerator-verify-<platform> …` | passes — contributes no token |
| Skill's only rule is `Bash(…/accelerator-verify *)` | raises for the fixture token |
| Scoped rule present, invocation `… <token> status && rm -rf x` | raises, `"declares no Bash(...) rule"` — kills a dropped `has_metacharacter` skip; the permissive scan still records the token, so the message names the rule, not a missing invocation |
| Skill invokes `cd . && …/accelerator zz-unregistered-zz` | raises, `"neither dispatched"` — the permissive fail-closed scan |
| Exempt token invoked mid-chain (`cd . && …/accelerator <exempt>`) | raises, `"exempt but"` |
| **Two**-token collection, one exempt with no consumer, the other bound | passes |
| Same token, exemption removed | raises, `"no skill invokes"` |
| Exemption naming a token absent from the collection | raises, `"exempt but not dispatched"` |
| Exemption set equal to the collection | raises, `"every dispatched sub-binary is exempt"` |
| Collection containing `verify` | raises, `"reserved"` |
| Collection containing `config` | raises, `"shadows a launcher built-in"` |
| Collection containing `frob_thing` | raises, `"not a valid token"` |
| Skill declares bare `Bash` and invokes the token | raises, `"declares no Bash(...) rule"` |
| Skill declares bare `Bash` **and** a correctly scoped rule | raises — bare `Bash` disqualifies the skill |
| Real repo, `exempt=()` passed explicitly | passes — the visualiser binding still holds |

The exemption pass-case injects a **two**-token collection: with one token and
one exemption, `set(tokens) <= set(exempt)` fires first, so a single-token
exemption can never pass. That derived rule — an exemption requires at least one
non-exempt dispatched token — is stated in `violations`' docstring and in
checklist point 7.

The real-repo case passes `exempt=()` rather than defaulting, so no future
addition to `SKILL_EXEMPT_SUBBINARIES` can make the guard's one production
binding pass vacuously. Name it `test_the_real_skills_tree_passes`, mirroring
`test_skill_permissions.py:128` — it and `lint:dispatch-coherence:check` in
`build-system:check` are jointly the CI gate.

**Source-scan tests** in the same file. Each is an absence assertion, so each is
paired with a **positive control** — the same predicate run over a deliberately
violating string must report it — following `test_python_coverage.py`'s sentinel
idiom and `test_workflows.py:269-284`'s `_BAD_MUTATIONS`. A negative regex
otherwise passes whenever the pattern is wrong, and these scans carry two
acceptance criteria between them.

- `_VISUALISE_SKILL_RELATIVE` appears nowhere under `tasks/`.
- `tasks/shared/dispatch_coherence.py` contains no literal `visualiser`.
- It imports all nine of `LAUNCHER`, `preprocessor_commands`,
  `frontmatter_bash_rules`, `has_bare_bash`, `has_metacharacter`,
  `is_plugin_invocation`, `covered_by`, `launcher_token` and `BARE_LAUNCHER`
  from `tasks.shared.skill_parsing`; **uses** each at least once outside the
  import block; shadows none of them (no `^\s*<name> =` / `^\s*def <name>`,
  unanchored so a nested definition cannot evade it); and imports no
  underscore-prefixed name from that module.
- It contains no `fnmatch`, and no `re.compile` whose pattern mentions `Bash` or
  the `!`-preprocessor form. The predicate is scoped to `re.compile` arguments
  deliberately: the guard's own error messages contain the literal `Bash(...)`,
  and the case matrix asserts on that substring, so a blanket `Bash(` ban would
  redden against the module it guards. A blanket `re.compile` ban would likewise
  overshoot, since the guard legitimately owns `_TOKEN`.
- It imports nothing from `tasks.lint` or `invoke`.

**Signature-pinning tests**, parametrised over **both** entry points —
`violations` (which the PR-time lint leaf calls) and
`validate_dispatch_coherence` (which the release path calls):

- `inspect.signature(...).parameters["tokens"].default is
  DISPATCHED_SUBBINARIES`, and likewise for `exempt`.

  **Known limitation, recorded deliberately**: `SKILL_EXEMPT_SUBBINARIES` is
  `()` and CPython interns the empty tuple, so its identity assertion is
  degenerate while the set is empty. A source scan that both `def` lines
  literally contain `= SKILL_EXEMPT_SUBBINARIES` accompanies it, and is the
  non-degenerate half.

**Cross-language pins** in the same file:

- `BUILTIN_SUBCOMMANDS` against the clap `Command` enum. Slice
  `cli/launcher/src/launch/inbound/cli.rs` from `pub enum Command {` to the next
  line-anchored `^}` — the file declares nine other enums whose bare variants
  sit at the same indent, so a line-matched regex would over-collect — assert
  the raw variant set equals `{"Version", "Config", "External"}` as a sanity
  anchor, then assert `{version, config} | {help}` equals `BUILTIN_SUBCOMMANDS`.
  Add a mutation control: the same extractor over the text with `    Vcs,`
  inserted into the `Command` body must fail. Also assert no `Command` variant
  carries a `name`, `alias` or `visible_alias` attribute, since clap routes on
  the effective name and an alias would be invisible to a variant-based
  extractor. `is_root_help`'s arm is asserted to agree as a secondary check.
- `RESERVED_TOKENS` against `_CLI_RELEASE_BINARIES` **minus the dispatched
  set**: assert it equals `{name.removeprefix("accelerator-") for name in
  _CLI_RELEASE_BINARIES if name != "accelerator"} - set(DISPATCHED_SUBBINARIES)
  | {"launcher"}`. The subtraction is load-bearing: checklist point 8 instructs
  every author to add `accelerator-<token>` to `_CLI_RELEASE_BINARIES`, so
  without it the first sibling story reserves its own token and cannot register.
  `verify` is reserved because it is staged and *never* dispatched.
- `_TOKEN` against the launcher's `derive_override_var` rule: a small
  accept/reject table (`vcs`, `work-item`, `frob_thing`, `2fast`, `Vcs`)
  asserted to agree with `cli/launcher/src/launch/core.rs`'s documented
  constraint.

**File**: `tests/unit/tasks/test_manifest.py`
**Changes**: Add the release-call-site test beside the existing `emit_manifest`
tests (`:194-229`): patch `validate_dispatch_coherence` with a spy **and**
`tasks.manifest.sign_file`, then assert `assert_called_once_with()` — no
collection or exemption argument. Patching `sign_file` keeps the test key-free
and `minisign`-free, so it cannot degrade into a skip the way
`_require("minisign")` would.

Also patch `validate_dispatch_coherence` in the three existing
`TestEmitManifest` round-trip tests (`:198-248`). They already need `minisign`
and the built verify shim; without the patch they would additionally run the
stricter scan over the real `skills/**/SKILL.md`, so one broadened
`allowed-tools` rule would redden four tests across two files with nothing
pointing at a permission rule. The real-tree assertion belongs to the single
dedicated case in `test_dispatch_coherence.py`.

**File**: `tests/unit/tasks/test_build.py`
**Changes**: Delete `TestValidateDispatchCoherence` (`:118-148`) entirely,
including `test_both_absent_is_coherent`, whose assertion the anti-vacuity rule
reverses. Narrow the import at `:18` to `from tasks.shared.errors import
InvalidVersionError` — `DispatchCoherenceError` is used only inside the deleted
class, and ruff `F401` is not relaxed for `tests/**`.

### Success Criteria

#### Automated Verification

- [ ] Guard suite passes: `uv run pytest
      tests/unit/tasks/shared/test_dispatch_coherence.py -v`
- [ ] Release call site pinned: `uv run pytest tests/unit/tasks/test_manifest.py
      -v`
- [ ] New lint task green: `mise run lint:dispatch-coherence:check`
- [ ] Task wiring pinned: `uv run pytest tests/unit/tasks/test_mise.py -v`
- [ ] Reachable from what CI runs — the guard fires from `mise run
      build-system:check`, not only from a bare `mise run`
- [ ] Every entry point still imports: `uv run python -c "import tasks,
      tasks.build, tasks.lint, tasks.manifest"`
- [ ] Unit suite passes: `mise run test:unit:tasks`
- [ ] Integration suite passes: `mise run test:integration:tasks`
- [ ] Build-system checks pass: `mise run build-system:check`

#### Manual Verification

- [ ] `rg 'visualiser' tasks/shared/dispatch_coherence.py` returns nothing.
- [ ] `rg '_VISUALISE_SKILL_RELATIVE' tasks/` returns nothing.
- [ ] `skills/visualisation/visualise/SKILL.md` is unedited (`jj diff` shows no
      change to it) — the drafting-time permission assumption held.
- [ ] Temporarily broadening the visualiser's rule to
      `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator *)` reddens `mise run
      build-system:check` with a message naming that SKILL.md, and reverting
      restores green — the guard is non-vacuous against the real tree, not only
      against fixtures.

---

## Phase 3: Generalise the debug-archive stage

### Overview

Remove the last visualiser-shaped release stage: `debug_archive_path`,
`create_debug_archives` and the unconditional upload append. Fix the
`.gitignore` rule that is supposed to keep the archives out of the release
commit but does not, and extend the SLSA test into a coverage assertion derived
from the actual publish set.

### Changes Required

#### 1. The ignore rule and the leak guard

Verified in a scratch repo: a gitignore pattern with a mid-string separator is
anchored to its `.gitignore`'s own directory, so `bin/*.debug.tar.gz`
(`.gitignore:54`) matches **only** `<repo-root>/bin/` — not
`skills/visualisation/visualise/bin/`, which is where `create_debug_archives`
writes. `**/bin/*.debug.tar.gz` does match. (`git check-ignore` run from inside
a jj workspace is useless here: it reports a match on the parent repo's
`workspaces/` rule regardless of the path asked about.)

So today's archives are untracked *and* unignored, and `tasks/git.py:73`'s bare
`git add .` would sweep them into the pushed version-bump commit. Constraining
registry values to a `bin/` directory — as this phase does below — is only
meaningful once the rule actually covers them.

**File**: `.gitignore`
**Changes**: Change `:54` to `**/bin/*.debug.tar.gz`. Correct `:28`'s now-stale
claim that "the four `bin/*.debug.tar.gz` archives under this tree are tracked"
— they were untracked by commit `3cc4d1d601`.

**File**: `tasks/release.py`
**Changes**: Add `.debug.tar.gz` to `_ARTIFACT_MARKERS` (`:20`, currently
`(".sec", "dist/release/", "dist/")`) **and** change
`_assert_no_leaked_artifacts` (`:39`) to run `git status --porcelain -uall`.
Verified the flag is load-bearing: porcelain's default untracked mode collapses
a wholly-untracked directory to one line, and after commit `3cc4d1d601` nothing
is tracked under `skills/visualisation/visualise/bin/` — so if the ignore rule
ever regressed, git would report `?? skills/visualisation/visualise/bin/` and
the string `.debug.tar.gz` would never appear in the scanned text. The existing
`dist/` marker survives this only because it happens to be a directory prefix.
Defence in depth: the ignore rule is the primary control, this is the backstop
that does not depend on a pattern being right.

#### 2. The debug-archive registry

**File**: `tasks/shared/paths.py`
**Changes**: Add `from collections.abc import Mapping` and `from types import
MappingProxyType` to the imports (the file currently imports only `tomllib`,
`Path` and `Any`), and replace the hardwired `debug_archive_path` (`:79-80`)
with a token-keyed registry. `BIN_DIR` stays as the visualiser's entry.

The helper takes the *directory* its three siblings in this module take
(`cli_binary_path`, `subbinary_asset_path`, `vendored_shim_path`) rather than
the mapping — both call sites already hold the directory from iterating the
registry, so a lookup inside the helper would be redundant. It is **required**,
with no default: a default of `BIN_DIR` is only correct for the visualiser, so
`debug_archive_path("vcs", platform)` would silently file under the visualiser's
skill tree and still satisfy the `bin/` constraint. `cli_member_manifests` in
this same module already documents that reasoning for a required path.

```python
# Sub-binaries shipping a symbolication archive, and the committed tree each is
# staged into so the provenance glob covers it. Every value must be a `bin/`
# directory — `.gitignore`'s archive rule is `**/bin/*.debug.tar.gz`.
DEBUG_ARCHIVE_DIRS: Mapping[str, Path] = MappingProxyType(
    {"visualiser": BIN_DIR}
)


def debug_archive_path(token: str, platform: str, bin_dir: Path) -> Path:
    return bin_dir / f"accelerator-{token}-{platform}.debug.tar.gz"
```

`MappingProxyType` keeps the registry read-only, matching
`DISPATCHED_SUBBINARIES`' immutable tuple, so a test cannot mutate the release
registry for the rest of the session. Convert `_SUBBINARY_MANIFESTS`
(`tasks/manifest.py:51-53`) the same way in this change — two edits there, not
one: add `from types import MappingProxyType` (the module imports nothing from
`types`) and **re-annotate** the constant from `dict[str, Path]` to
`Mapping[str, Path]`, since `MappingProxyType` is not a `dict` subclass and
leaving the annotation would fail `types:build-system:check`.
`_default_subbinary_manifest`'s `.get()` is `Mapping`-compatible.

#### 3. The producer

**File**: `tasks/build.py`
**Changes**: Add `from collections.abc import Iterable, Mapping` — the file
currently imports **no** `collections.abc` names, so both are undefined — and
keep `DISPATCHED_SUBBINARIES` in the `tasks.shared.paths` import block. Phase 2
removes it "if nothing else in the file uses them"; this phase uses it, and the
two phases are freely orderable, so the import must be present after Phase 3
whichever lands first.

Split `create_debug_archives` (`:498-510`) into two pure helpers plus a thin
`@task`, following the `vendor_shim_marker_digest` / `vendor_verify_shims` split
already in this file. The `BIN_DIR` import is replaced by `DEBUG_ARCHIVE_DIRS`.

The `@task` stays **argument-free**. It is mise-wired as `build:debug-archives`
(`mise.toml:135-138`) and called from `prerelease_prepare` and
`release_prepare`, so a parameter would become an operator-facing CLI flag on a
release-path leaf — and invoke derives an argument's kind from its default, so a
`Mapping[str, Path]` default is not expressible on a command line at all. The
seams live on the helpers, which is the same rule Phase 4 states for
`upload_and_verify_release`.

```python
def _debug_archive_targets(
    dirs: Mapping[str, Path] = DEBUG_ARCHIVE_DIRS,
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
    staging_dir: Path = RELEASE_STAGING,
) -> list[tuple[Path, Path]]:
    """Each (staged binary, archive path) pair the debug archives cover."""
    unknown = sorted(set(dirs) - set(tokens))
    if unknown:
        raise RuntimeError(
            f"debug-archive registry names undispatched token(s): {unknown} — "
            "nothing cross-compiles them, so the archive source would be absent"
        )
    stray = sorted(str(d) for d in dirs.values() if d.name != "bin")
    if stray:
        raise RuntimeError(
            f"debug-archive directories must be `bin/` trees: {stray} — "
            "`.gitignore`'s archive rule is `**/bin/*.debug.tar.gz`, so an "
            "archive written elsewhere would be committed by `git add .`"
        )
    return [
        (
            subbinary_asset_path(token, platform, staging_dir),
            debug_archive_path(token, platform, directory),
        )
        for token, directory in dirs.items()
        for _triple, platform in TARGETS
    ]


def _write_debug_archives(targets: list[tuple[Path, Path]]) -> None:
    for binary, archive in targets:
        archive.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(binary, arcname=binary.name)


@task
def create_debug_archives(context: Context) -> None:
    """Archive each cross-compiled sub-binary that ships symbolication data.

    Archives the shared dist/release binary into the sub-binary's committed
    tree, where the provenance glob covers it.
    """
    _write_debug_archives(_debug_archive_targets())
```

`RuntimeError` rather than `ValueError`: `tasks/build.py` raises `RuntimeError`
at every one of its nine non-typed precondition sites and contains no
`ValueError`, so this matches the module. Both raises are programming errors in
a constant, not release-flow conditions, which is why they take no typed error
from `tasks/shared/errors.py`.

#### 4. The upload

**File**: `tasks/github.py`
**Changes**: Add `DEBUG_ARCHIVE_DIRS` to the `tasks.shared.paths` import block
(`:15-24`), which does not name it today. `:224` becomes a loop over the
registry. Iteration order is preserved exactly — debug archives, then launcher,
then launcher signature, per platform — so the committed `assert len(uploads) ==
22` pin (`test_github.py:326`) and the ordering it implies stay green.

```python
    for _triple, platform in TARGETS:
        for token, directory in DEBUG_ARCHIVE_DIRS.items():
            uploads.append(debug_archive_path(token, platform, directory))
        launcher = cli_binary_path("accelerator", platform)
        uploads.append(launcher)
        uploads.append(_sig(launcher))
```

This phase reads the module constant directly; `_release_uploads` gains its
`debug_dirs` parameter in Phase 4, alongside `tokens`. Phases 3 and 4 are
declared freely orderable, so Phase 3 must not reference a name Phase 4
introduces.

Two stale comments are rewritten in token-generic terms: `:220-222` ("The
visualiser binary is published once…") and the section header at `:150`
("unified launcher + manifest + visualiser publish"), which is the second
`visualiser` literal in this file and the reason the phase's grep criterion
would otherwise fail.

#### 5. The SLSA guard

**File**: `.github/workflows/main.yml`
**Changes**: Add `dist/release/manifest.json` and
`dist/release/manifest.minisig` to the `subject-path` list of all three
`attest-build-provenance` blocks (`:423`, `:534`, `:556`). Verified: each block
lists only `skills/visualisation/visualise/bin/accelerator-visualiser-*` and
`dist/release/accelerator-*`, while `_release_uploads` also uploads
`RELEASE_MANIFEST` and `RELEASE_MANIFEST_SIG` — so the signed manifest, the one
document naming every sub-binary's sha256 and inline signature, ships today with
**no** build provenance. Verified the ordering is safe: each attest step runs
after its `Sign*` step and before its `Finalise*` step, so both files exist.

**File**: `tests/unit/tasks/test_workflows.py`
**Changes**: Extend the existing
`test_attest_globs_include_the_launcher_binaries` (`:147-159`) rather than
adding a weaker sibling — it already iterates every attest step and asserts both
globs. Four assertions:

- **Count, not `assert attest_steps`**: one attestation per `Sign*` step per
  job. The file already enumerates `Sign*` steps at `:127`. Verified to yield 1
  for the `prerelease` job and 2 for the `release` job, so it survives a fourth
  release lane where a hardcoded `== 3` would not, and it fails if the stable
  track's attest step is deleted, which `> 1` does not.
- **Ordering**: in each job every attest step's index is greater than its paired
  `Sign*` step's and less than the following `Finalise*` step's — otherwise a
  future reordering could publish before attesting, or publish and then fail
  before attesting, leaving assets permanently unattested.
- **Symmetry**: the parsed `subject-path` sets are equal across all blocks.
- **Coverage derived from the publish set**: every path in `_release_uploads()`,
  expressed relative to `REPO_ROOT`, is matched by some `subject-path` glob in
  every block. Assert the derived path set is non-empty first so the loop cannot
  become a silent no-op. Model `@actions/glob` semantics (`*` does not cross
  `/`) rather than `fnmatch`, whose `*`-spans-`/` behaviour would make the
  assertion over-permissive for a nested staging tree. This replaces the
  hardcoded `accelerator-visualiser-*` literal with a derived one, so the
  no-literal-`visualiser` goal cannot tempt anyone into deleting the only
  assertion covering the archives.

#### 6. Tests

**File**: `tests/unit/tasks/test_build.py`
**Changes**: Test `_debug_archive_targets` against an injected two-token
registry of `tmp_path` `bin/` directories **plus a matching two-token
collection**, asserting one pair per token per target with the
`accelerator-<token>-<platform>.debug.tar.gz` name; and that a non-`bin`
directory, and a registry key absent from the injected collection, each raise
`RuntimeError`. Test `_write_debug_archives` against those pairs with fake
staged binaries under `tmp_path`, asserting one archive per pair and that each
directory is created. Both are pure functions, so neither test patches module
state nor writes into the committed skill tree.

Also narrow `:19`'s `from tasks.shared.paths import cli_binary_path,
vendored_shim_path` once `TestCliPathHelpers` moves out (below), and remove the
now-orphaned `# ── cli path helpers ──` banner at `:115`. Both are
`F401`/tidiness consequences of the move that ruff will otherwise flag.

**File**: `tests/unit/tasks/shared/test_paths.py` (new)
**Changes**: Test `debug_archive_path` for an injected directory. Move
`TestCliPathHelpers` (`tests/unit/tasks/test_build.py:174-186`) here in the same
change, so `tasks/shared/paths.py` coverage lives in one file under the
`tests/unit/tasks/shared/` mirror. The moved class asserts against `_REPO_ROOT =
Path(__file__).resolve().parents[3]` (`test_build.py:22`), which is off by one a
directory deeper — resolve the root with `repo_root()` from
`tasks.shared.sources` instead, since no existing file under
`tests/unit/tasks/shared/` computes a repo root and there is no local idiom to
copy.

Add a single literal pin — `assert dict(DEBUG_ARCHIVE_DIRS) == {"visualiser":
BIN_DIR}` — the same anti-vacuity anchor `DISPATCHED_SUBBINARIES` gets. It is
the only control against an emptied registry: `_debug_archive_targets` raises on
an undispatched key and a non-`bin` value but not on an empty mapping, and the
SLSA coverage loop's non-empty guard is satisfied by the launchers and manifest
alone.

**File**: `tests/integration/tasks/test_release.py`
**Changes**: Add a `test_fires_on_a_debug_archive` case to
`TestLeakedArtifactGuard` (`:194`; `dist/` has no case today), and a `pathspec`
test asserting the ignore rule actually matches — following
`tests/unit/tasks/test_bootstrap_coverage.py:79-92`, which already uses
`pathspec.GitIgnoreSpec.from_lines`:

```python
spec = pathspec.GitIgnoreSpec.from_lines(
    (REPO_ROOT / ".gitignore").read_text().splitlines()
)
archive = debug_archive_path("visualiser", "linux-x64", BIN_DIR)
assert spec.match_file(archive.relative_to(REPO_ROOT).as_posix())
```

Without this the `.gitignore` correction and its `_ARTIFACT_MARKERS` backstop —
the primary control and its backstop for keeping a release artefact out of the
pushed version-bump commit — both land untested, and a future tidy-up that
re-narrows the pattern would be caught by nothing. Phase 5's checklist test
asserts the literal string appears, which is text presence, not matching
semantics.

**File**: `tests/integration/tasks/test_github.py`
**Changes**: `_setup_release`'s `debug_archive_path` double (`:252-258`) is a
**one-argument** lambda and must move to the new signature, mirroring the
`subbinary_asset_path` double directly below it (`:264-268`):

```python
    mocker.patch.object(
        gh,
        "debug_archive_path",
        side_effect=lambda token, p, _dir: (
            tmp_path / f"accelerator-{token}-{p}.debug.tar.gz"
        ),
    )
```

Without this every test in `TestUploadAndVerifyRelease` raises `TypeError`,
including the `assert len(uploads) == 22` pin this phase relies on as its
regression evidence.

### Success Criteria

#### Automated Verification

- [ ] Debug-archive tests pass: `uv run pytest tests/unit/tasks/test_build.py -k
      debug -v`
- [ ] Path-helper tests pass: `uv run pytest
      tests/unit/tasks/shared/test_paths.py -v`
- [ ] SLSA coverage test passes: `uv run pytest
      tests/unit/tasks/test_workflows.py -v`
- [ ] The 22-upload pin still holds, after the `_setup_release` fixture update:
      `uv run pytest tests/integration/tasks/test_github.py -v`
- [ ] Unit + integration suites pass: `mise run test:unit:tasks && mise run
      test:integration:tasks`
- [ ] Build-system checks pass: `mise run build-system:check`

#### Manual Verification

- [ ] `rg 'visualiser' tasks/github.py` returns nothing — both `:150` and
      `:220-222` rewritten.
- [ ] `rg "visualiser" tasks/shared/paths.py` matches exactly: `VISUALISER`
      (`:8`) and its comment, `SERVER` (`:14`), `FRONTEND` (`:16`),
      `subbinary_asset_path`'s docstring (`:67-68`), and the two registry
      constants with their comments — the visualiser's own paths, which this
      work does not touch.
- [ ] The ignore fix works: in a scratch clone, `git check-ignore -v
      skills/visualisation/visualise/bin/x.debug.tar.gz` matches
      `**/bin/*.debug.tar.gz`. Run it outside this jj workspace, where the
      parent's `workspaces/` rule would mask the answer.
- [ ] Pointing a second `DEBUG_ARCHIVE_DIRS` entry at a non-`bin` directory
      raises from `_debug_archive_targets`; pointing it at a second `bin/` tree
      reddens the SLSA coverage test until `main.yml` gains the sibling
      `subject-path` line in all three blocks.

---

## Phase 4: Injectable seams on the release-stage builders

### Overview

Give the three builders injected token collections so the "already
parameterised" assumption is discharged by test. Signature changes only; each
default keeps today's behaviour.

All four seams — including Phase 3's `_debug_archive_targets` — use **one**
idiom: a direct default of the real constant, annotated `Iterable[str]` per
`collect_entries` (`tasks/manifest.py:69`). That is required by the guard's
identity-pinning criterion and is the only shape five sibling stories can follow
without knowing which module they are in.

Two boundaries on that rule, both load-bearing:

- **Seams go on private helpers; every `@task` stays argument-free.**
  `upload_and_verify_release` (`tasks/github.py:300`) and
  `create_debug_archives` are both invoke `@task`s, so a parameter becomes an
  operator-facing CLI flag on a release-path leaf — and invoke derives an
  argument's kind from its default, so a `Mapping[str, Path]` default is not
  expressible on a command line at all. Every other injection seam in this
  codebase sits on a pure helper with the `@task` calling it argument-free
  (`collect_entries`, `cli_member_manifests`, `debug_archive_path`). Tests drive
  the helpers directly.
- **The rule covers token collections only.** `RELEASE_MANIFEST` and
  `RELEASE_MANIFEST_SIG` stay module globals read at call time, and
  `test_github.py:271`/`:272`'s patches stay with them. A def-time
  `manifest_path: Path = RELEASE_MANIFEST` default would bind the real path at
  import, so the `:271` patch would stop reaching it — `_release_uploads` would
  list the real `dist/release/manifest.json`, the `missing` check at `:314`
  would raise `FileNotFoundError`, and every test in
  `TestUploadAndVerifyRelease` would fail, including the 22-upload pin.
- **The `@task` resolves the token collection once and threads it.**
  `upload_and_verify_release` reads `DISPATCHED_SUBBINARIES` from module scope
  at call time and passes it to both `_release_uploads(tokens)` and
  `_release_reverifies(context, tag, tokens)`. This is load-bearing twice over.
  It gives the resolve-once property — the "every asset uploaded" and "every
  asset re-verified before `--draft=false`" lists cannot derive from different
  values. And it is what keeps `test_github.py:427`'s `mocker.patch.object(gh,
  "DISPATCHED_SUBBINARIES", ("foo",))` working: a def-time default on a builder
  is bound at import and a module patch cannot reach it, but the `@task` reading
  the global at call time can. The builders keep their direct defaults for the
  acceptance criterion's argument-free case and for the seam tests, so all four
  seams share one idiom and no `None` sentinel appears.

`collect_entries`' first parameter is renamed `subbinaries` → `tokens` in the
same change (positional at both call sites, so the edit is local), so the
precedent the plan cites and the seams it adds use one word for one thing.

The Test-seams *fallback* in the work item is **not** taken: the
`sign_staged_binaries` extraction is confined to the expected-set construction
(`tasks/signing.py:62-66`, a two-line comprehension) and does not touch the
signing flow.

### Changes Required

#### 1. Signing

**File**: `tasks/signing.py`
**Changes**: Add `Iterable` to the `collections.abc` imports (`:5`, currently
`Iterator`). Extract the sub-binary half of `expected` (`:62-66`) into a pure
helper, named `_subbinary_signing_targets` for symmetry with
`_subbinary_uploads`, `_subbinary_reverifies` and the existing `_signature_path`
— nothing outside this module and its test calls it. Per the acceptance
criterion's "4 unsigned binary paths" figure, it returns **only** the sub-binary
paths, not the four launcher paths.

```python
def _subbinary_signing_targets(
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
) -> list[Path]:
    return [
        subbinary_asset_path(token, platform)
        for _triple, platform in TARGETS
        for token in tokens
    ]
```

`sign_staged_binaries` becomes `expected += _subbinary_signing_targets()`.

#### 2. Uploads

**File**: `tasks/github.py`
**Changes**: Add `Iterable` and `Mapping` to the `collections.abc` imports
(`:5`, currently `Callable`). Extract the sub-binary loop (`:230-234`) into its
own helper so the criterion's "8 upload entries" is a direct assertion rather
than a filter over the 22-entry whole.

```python
def _subbinary_uploads(
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
) -> list[Path]:
    uploads: list[Path] = []
    for token in tokens:
        for _triple, platform in TARGETS:
            asset = subbinary_asset_path(token, platform)
            uploads.append(asset)
            uploads.append(_sig(asset))
    return uploads
```

`_release_uploads` gains the same `tokens` parameter **and** `debug_dirs:
Mapping[str, Path] = DEBUG_ARCHIVE_DIRS` — Phase 3 reads the module constant
directly, so the parameter is introduced here — and threads both on. The loop
variable is `token` throughout, not `name`: the change exists to establish
"token" as the domain term.

#### 3. Re-verification

**File**: `tasks/github.py`
**Changes**: `_subbinary_reverifies` (`:270-293`) gains `tokens` as a direct
default; `_release_reverifies` gains it too and threads it on, so the "upload
every asset" and "re-verify every asset before `--draft=false`" lists derive
from one value rather than two. `RELEASE_MANIFEST` continues to be read at call
time.

The existing `if not names: return []` **stays**. An earlier draft turned it
into a raise, which was wrong twice over: `_release_reverifies` is called at
`tasks/github.py:319`, one line above the `try` at `:320`, so a raise there
escapes both handlers — no forensic alert, no draft-preserve, no `--cleanup-tag`
— after `commit_version`, `tag_version`, `push` and `create_release` have run;
and anti-vacuity on the token collection is already owned by the guard's
`_registry_problems`, which fires at sign time. A second copy here would restate
that invariant at the worst possible point in the flow.

```python
def _subbinary_reverifies(
    context: Context,
    tag: str,
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
) -> list[_Reverify]:
    names = tuple(tokens)
    if not names:
        return []
    manifest = json.loads(RELEASE_MANIFEST.read_text())
    ...
```

#### 4. The manifest/token agreement

The check that the signed manifest and the staged token set describe the same
binaries belongs in `_publish`, not in `emit_manifest`.

An `emit_manifest` check cannot fire: at its only call site
(`tasks/release.py:_sign`) `entries` comes from `manifest.collect_entries()` and
both it and `emit_manifest` would default to `DISPATCHED_SUBBINARIES`, so the
two operands have one source and the comparison is a tautology. Worse, all three
committed `TestEmitManifest` tests pass entries that could not satisfy it (`{}`,
`{}` and `collect_entries(["foo"], …)`), so it would redden them.

`_publish` reads the manifest back **from disk**, which is where a real
divergence lives: `release:finalise` and `prerelease:finalise` are separately
invocable mise leaves, so a `dist/release/` left from an earlier cut is
reachable. And the check lands before anything irreversible.

**File**: `tasks/release.py`
**Changes**: Add `import json`, and add `DISPATCHED_SUBBINARIES` to the existing
`from .shared.paths import RELEASE_MANIFEST` line — the module imports neither
today. Extract a named helper beside `_assert_no_leaked_artifacts`, matching how
that module already expresses a `_publish` precondition, and call it from
`_publish` between `_assert_no_leaked_artifacts(context)` and
`git.commit_version(context)`:

```python
def _assert_staged_manifest_is_current(version: str) -> None:
    if not RELEASE_MANIFEST.exists():
        raise RuntimeError(
            f"{RELEASE_MANIFEST} is absent — run the prepare and sign steps "
            "before finalise"
        )
    manifest = json.loads(RELEASE_MANIFEST.read_text())
    listed = set(manifest["binaries"])
    if listed != set(DISPATCHED_SUBBINARIES):
        raise RuntimeError(
            f"staged manifest lists {sorted(listed)} but this release "
            f"dispatches {sorted(DISPATCHED_SUBBINARIES)} — a signed manifest "
            "promising an asset that was never uploaded cannot be recalled"
        )
    if manifest["version"] != version:
        raise RuntimeError(
            f"staged manifest is version {manifest['version']} but this "
            f"release is {version} — dist/release/ is from an earlier cut; "
            "re-run the prepare and sign steps"
        )
```

The **version** comparison is what catches the stale-cut scenario; the token-set
comparison alone does not. `dist/release/` is never cleaned (only
`mkdir(exist_ok=True)` at `tasks/build.py:299`/`:319`), and an earlier cut has
the *same* token set — the registry changes once per sub-binary story — so a
stale manifest would pass a key-set check and reach `commit_version`,
`tag_version` and `push`. `_publish` already computes `resolved_version` on its
first line, and `validate_version_coherence` runs only inside `emit_manifest`,
which `*:finalise` does not execute.

`RuntimeError` matches `_assert_no_leaked_artifacts` beside it. It deliberately
does not reuse `AssetVerificationError`, whose meaning throughout
`tasks/github.py` is "a published-candidate asset failed its sha256 or minisign
check → preserve the draft for triage".

**File**: `tests/integration/tasks/test_release.py`
**Changes**: Three committed tests drive `_publish` with a `MagicMock` context
and no staged manifest —
`TestPrereleaseFinalise::test_publish_calls_unified_upload` (`:176`),
`::test_commits_before_upload` (`:182`) and
`TestLeakedArtifactGuard::test_publish_runs_the_guard_before_commit` (`:215`).
All three reach the new read, so each must patch `tr.RELEASE_MANIFEST` at a
`tmp_path` manifest whose `binaries` set is `{"visualiser"}` and whose `version`
matches, mirroring `_setup_release`'s `mocker.patch.object(gh,
"RELEASE_MANIFEST", …)` idiom (`tests/integration/tasks/test_github.py:271`).
Patch the *manifest*, not the new helper: patching the helper out would disable
it inside `test_publish_runs_the_guard_before_commit`, the only test asserting
the pre-commit guards run before `commit_version`.

Without this the phase lands red in CI and passes only on a machine that has run
a local release — the same audit the plan performs for the rejected
`emit_manifest` placement.

#### 5. Tests

**File**: `tests/unit/tasks/test_signing.py`
**Changes**: With a two-token fixture collection injected,
`_subbinary_signing_targets` yields one path per token per target across the
four `TARGETS`. Called with no argument, it yields exactly `len(TARGETS)` paths,
every one naming `visualiser`. No signing key required — pure list derivation.

Add one test for `sign_staged_binaries` itself, which has no test at any level
today: with `sign_file` patched and the eight paths staged under `tmp_path`,
assert the expected set is the four launcher paths **plus** the four sub-binary
paths, and that a missing one raises `SigningError`. Without it, mutating
`expected += …` to `expected = …` — dropping the launcher binaries — passes
every test this phase adds.

**File**: `tests/integration/tasks/test_github.py`
**Changes**: With a two-token fixture collection, `_subbinary_uploads` yields an
asset **and** its `.minisig` per token per target; with no argument, one pair
per target, every one naming `visualiser`. `_subbinary_reverifies`, given a
two-token in-test manifest via the existing `RELEASE_MANIFEST` patch, yields one
item per token per target; with no argument against the `_setup_release`
manifest, one per target. No network — the `_Reverify` items are constructed,
never invoked.

**Leave `:271`, `:272` and `:427`'s module patches alone.** Two earlier drafts
got this wrong in opposite directions — one migrated `:427` to an argument, the
other kept the patch while giving the builder a def-time default a module patch
cannot reach. Both break `test_includes_subbinary_assets_when_present`
(`:403-433`), which asserts on `gh release upload … accelerator-foo-… --clobber`
strings produced by `_release_uploads` **through the `@task`**. The resolve-once
threading above is what makes the patch reach the builder, so the test stands
unchanged and remains the suite's only end-to-end evidence that a registered
token reaches a real upload invocation. The new builder tests pass arguments
directly.

Derive expected counts from `len(TARGETS)` and the injected collection rather
than hardcoding 8 and 16. For the **default** calls, derive the expectation from
the module constant `DISPATCHED_SUBBINARIES` — never from
`inspect.signature(...).default` or from the function's own output, either of
which would make an empty-tuple default yield `expected == actual == 0` and
discharge the criterion vacuously — and keep the "every path names `visualiser`"
assertion alongside the count so a wrong constant fails independently of the
arithmetic. Keep exactly **one** literal pin of the registry: `assert
DISPATCHED_SUBBINARIES == ("visualiser",)` in a single named test.

Keep `assert len(uploads) == 22` literal through Phases 3 and 4 as the
regression anchor. Replacing it with a derived expression is a follow-up once
the first sibling token lands, not part of this change.

**File**: `tests/integration/tasks/test_release.py`
**Changes**: `_publish` raises `RuntimeError` when the staged manifest's
`binaries` set differs from `DISPATCHED_SUBBINARIES` in either direction, and
does not raise when they agree.

A mis-wired default (empty tuple, wrong constant, mutable-default aliasing)
fails these rather than silently emptying the real release.

### Success Criteria

#### Automated Verification

- [ ] Signing seam tests pass: `uv run pytest tests/unit/tasks/test_signing.py
      -v`
- [ ] Builder seam tests pass: `uv run pytest
      tests/integration/tasks/test_github.py -v`
- [ ] Manifest agreement tests pass: `uv run pytest
      tests/unit/tasks/test_manifest.py -v`
- [ ] The 22-upload pin and the migrated second-token injection are both green
      in the run above
- [ ] Unit + integration suites pass: `mise run test:unit:tasks && mise run
      test:integration:tasks`
- [ ] Build-system checks pass: `mise run build-system:check`

#### Manual Verification

- [ ] A release dry-run's staged asset set is unchanged from before the phase
      (compare `dist/release/` listing against a pre-change run).
- [ ] `rg 'is not None' tasks/github.py tasks/signing.py` shows no
      token-collection sentinel — one idiom across all four seams.
- [ ] `test_includes_subbinary_assets_when_present` is unmodified and green —
      the resolve-once threading is what lets its module patch reach the
      builder.
- [ ] `invoke --help github.upload-and-verify-release` lists only `version` — no
      `tokens` or `manifest-path` flag leaked onto the publish task.

---

## Phase 5: The registration checklist

### Overview

Document the registration surface in `tasks/README.md` as thirteen numbered
points, each opening with an imperative action line or carrying the literal
phrase `No action when`, pinned by a test that checks both the README's own text
and that each named registration point still resolves in the source file the
checklist attributes it to.

### Changes Required

#### 1. The section

**File**: `tasks/README.md`
**Changes**: Add `## Registering a dispatched sub-binary` **immediately before**
`## CI job → local command` (`:148`) — i.e. after the Contributor environment
variables subsection. Inserting it after the `## Conventions (learn once)`
heading at `:50` would reparent that section's three `###` subsections
(Executable-bit invariant `:68`, Rust nightly lane `:101`, Contributor
environment variables `:137`) under the new heading. Extend the file's opening
sentence to mention that it also carries this checklist, since it is no longer
only a description of the task tree's shape.

Open the section with a short lead-in, because the audience arrives by following
the anchor from a sibling work item and lands mid-file — and because
`tasks/README.md` today uses neither "token" nor "sub-binary" anywhere:

> A **dispatched sub-binary** is a separate static binary the launcher fetches
> on demand from the signed release manifest (ADR-0054). Its **token** is the
> subcommand name in `accelerator <token>`, and the same string is also the
> `DISPATCHED_SUBBINARIES` entry, the manifest key, and the launcher's cache
> filename prefix — while the crate, its `[[bin]]` and the published asset all
> carry an `accelerator-` prefix. The registries are spelled `*_SUBBINARIES` for
> history. Each point below is tagged with where a mistake surfaces: **[PR]** a
> test or a per-PR CI gate catches it, **[release]** it fails the release job,
> **[author]** nothing catches it.

Thirteen numbered points. Each opens with a verb from the closed set the test
enforces — **Add**, **Register**, **Update**, **Document**, **Extend** — or
carries `No action when`, and each conditional point states its condition in the
same clause as the verb:

1. **Add** the token to `DISPATCHED_SUBBINARIES` (`tasks/shared/paths.py`), then
   update three things in `tests/integration/tasks/test_github.py`: the registry
   pin (a deliberate anti-vacuity anchor, not a count to bump blindly), the
   `assert len(uploads) == 22` count, and `_setup_release`'s staged fixture and
   in-test manifest, which are single-token today. Converting that count to a
   derived expression is expected of the first sibling to land. **[PR]**
2. **Add** an entry to `_SUBBINARY_MANIFESTS` (`tasks/manifest.py`) when the
   crate is not at `cli/<token>/`. The visualiser is the worked example:
   `"visualiser": CLI_DIR / "visualiser/server/Cargo.toml"`. **No action when**
   the crate is at `cli/<token>/`. **[release]**
3. **Add** the crate's `Cargo.toml`: `[[bin]] name = "accelerator-<token>"` (the
   asset name the manifest and signing expect), a mandatory
   `package.description` (the manifest sources the description from it), and the
   inherited `version.workspace`, `edition.workspace`, `rust-version.workspace`,
   `license.workspace` and `publish.workspace`. Inherit the version so the next
   workspace bump cannot desynchronise the member — the version-coherence check
   in `tasks/build.py` only reports a *mismatch*, so a hardcoded current version
   passes today and breaks at the next bump. For lints, either inherit with
   `[lints] workspace = true` or declare a crate-local `[lints.clippy]` table if
   you need allows, as `cli/visualiser/server/Cargo.toml` does; either way `-D
   warnings` from `lint:cli:check` still promotes warnings to errors, and
   without the workspace table you opt out of the shared pedantic/nursery/
   `unwrap_used` opt-ins rather than out of lint enforcement. **[release]**
4. **Register** the crate in `[workspace].members` in `cli/Cargo.toml`
   (**[release]** — nothing pins the members list, so an omission surfaces as a
   missing cross-compiled binary during signing), and commit the regenerated
   `cli/Cargo.lock` (`lint:cli:check` runs `--locked`, so a stale lock reddens
   `cli:check` as an apparent clippy failure).
5. **Add** `bin/<token>-*` to `.gitignore`. The launcher caches every fetched
   sub-binary as `bin/<token>-<version>-<sha256>` in the plugin root, so this is
   needed whether or not anything is staged there at release time.
   `bin/*.minisig` and `**/bin/*.debug.tar.gz` are already token-generic.
   **[author]** — nothing catches it, and the local-dev release path's `git add
   .` would sweep the cached binary into the version-bump commit.
6. **Add** an entry to `cli/launcher/tests/fixtures/manifest.example.json` only
   if you want the golden contract to stay representative of a multi-binary
   manifest. **No action when** you are adding a new key: both co-readers —
   `tests/unit/tasks/test_manifest_contract.py`, which iterates
   `binaries.values()` generically, and the `include_str!` in
   `cli/launcher/src/launch/outbound/resolve/manifest.rs`, which reads the
   existing entry — are key-agnostic and break only if an existing entry is
   renamed or removed. **[author]**
7. **Add** the skill binding: a skill invoking `accelerator <token>` through the
   `!` preprocessor, plus a `Bash(...)` rule whose subcommand segment is exactly
   `<token>` and which covers that invocation. A rule scoped tighter than the
   token (`Bash(…/accelerator <token> start)`) binds provided it covers the
   invocation; a wildcarded segment (`Bash(…/accelerator <tok>*)`) does not. A
   bare `Bash` tool, a rule authorising the bare launcher, or a rule with a
   wildcarded token segment **anywhere** in that skill's frontmatter
   disqualifies the whole skill as a witness, so pick or write a skill that has
   none of them. The guard sees only `!`-preprocessor commands in
   `skills/**/SKILL.md` naming `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` —
   invocations from `hooks/`, `scripts/` and model-driven Bash are outside its
   reach. **[PR]**
   - Also check: if the binding is satisfied by a *new* skill that injects
     config context or instructions, bump `EXPECTED_INJECTION_SKILLS`
     (`tasks/lint/skill_permissions.py`) in the same change, and keep the `!`
     command free of shell metacharacters — both are separate guards. Authoring
     a new skill is its own registration surface; this checklist covers only the
     binding.
   - Alternatively **add** an entry to `SKILL_EXEMPT_SUBBINARIES` when *no*
     SKILL.md invokes the token. An exemption whose token is invoked is
     rejected, as is one naming an undispatched token, and at least one
     dispatched token must remain non-exempt.
8. **Add** `accelerator-<token>` — the cargo `[[bin]]` name from point 3, not
   the bare token — to `_CLI_RELEASE_BINARIES` (`tasks/build.py`).
   `cli_cross_compile` stages via `cli_binary_path(name, platform)`, i.e.
   `dist/release/<name>-<platform>`, which equals `subbinary_asset_path(token,
   platform)` **only** because of that prefix; a bare token stages
   `dist/release/<token>-<platform>` and signing then fails on a missing
   `accelerator-<token>-<platform>`. It also gives you `_assert_magic_bytes`
   and, for musl, `_assert_static_elf`, and `build:cli:cross-compile` is already
   called from `prerelease_prepare` and `release_prepare` (`tasks/release.py`).
   **No action when** you take that route — no new task and no `mise.toml` leaf
   are needed. A crate that cannot ride that loop needs its own staging task
   wired into **both** prepare tasks *and* a `mise.toml` leaf, and owes
   `_assert_static_elf` explicitly for musl. **[release]**
9. **Update** all three `attest-build-provenance` blocks in
   `.github/workflows/main.yml`, identically, **only when** the release
   publishes an artefact that no existing `subject-path` glob matches — today, a
   symbolication archive written into a committed `bin/` tree (point 12). The
   condition is about the published *artefact*, not the sub-binary: the
   sub-binary is always staged in `dist/release/`, which
   `dist/release/accelerator-*` covers — but `manifest.json` and
   `manifest.minisig` live there too and were matched by nothing until Phase 3
   added them, which is why the rule is stated this way. **[PR]** — a test
   derives the expected coverage from `_release_uploads()`.
10. **Update** `BUILTIN_SUBCOMMANDS` (`tasks/shared/dispatch_coherence.py`) in
    lockstep whenever the launcher's built-in set changes in either direction.
    **No action when** you are only adding a dispatch token — but note a name in
    that set is unavailable as one. A test pins the set against the clap
    `Command` enum, so the two cannot drift. **[PR]**
11. **Document** the sub-binary for users: its own page under `docs/`, an entry
    in the **Concepts** list under `## Documentation` in the root `README.md`,
    and an `ACCELERATOR_<TOKEN>_BIN` override row wherever that sub-binary's
    overrides are documented (`docs/visualiser.md` is the visualiser's).
    `docs/*.md` form a hand-maintained prev/next chain, so inserting a page
    means editing the footer of the page **before** and the page **after** it as
    well as your own. `docs/internals.md`'s env-var table holds only
    launcher-wide inputs and is already token-generic. **No action when** the
    sub-binary is not user-facing. **[author]**
12. **Add** an entry to `DEBUG_ARCHIVE_DIRS` (`tasks/shared/paths.py`) when the
    sub-binary ships a symbolication archive, and update the registry pin in
    `tests/unit/tasks/shared/test_paths.py`. The value must be a `bin/`
    directory — `.gitignore`'s rule is `**/bin/*.debug.tar.gz`, so an archive
    written elsewhere would be committed by the release path's `git add .`. This
    is what triggers point 9's obligation. **No action when** the sub-binary
    ships no symbolication archive — omitting an entry silently ships no archive
    and nothing catches it, though the shape of an entry you *do* add is checked
    by `_debug_archive_targets`. **[author]**
13. **Extend** `cli/deny.toml` when the new crate's dependency graph needs a
    licence or advisory exception, with a comment giving the justification. **No
    action when** `mise run deny:check` is already green. **[PR]**

Plus, stated in the section body:

- Points 1, 2, 3, 4, 7 and 8 must land in the **same change**. The release path
  resolves them together, and only the 1↔7 pair is caught before the release job
  — by the dispatch guard, which runs from `tasks/manifest.py` on every release
  *and* as `lint:dispatch-coherence:check` in `build-system:check`.
- The Cargo **package** is `accelerator-<token>`; where a domain crate already
  owns `cli/<token>/`, the binary crate lives elsewhere with a
  `_SUBBINARY_MANIFESTS` entry, because `tasks/manifest.py` defaults the
  manifest path to `cli/<token>/Cargo.toml` and cargo-pup rules match on whole
  crate names. A crate carrying domain modules may also owe a `cli/pup.ron`
  rule; that is the generic add-a-Rust-crate surface, not part of this
  checklist.
- The **token** must match `^[a-z][a-z0-9-]*$`. Underscores are rejected because
  the token derives `ACCELERATOR_<TOKEN>_BIN`
  (`cli/launcher/src/launch/core.rs`), which the launcher refuses to build from
  a name outside that set — so an underscore token can never resolve an
  override.
- `verify` and `launcher` are **reserved**, and a name in `BUILTIN_SUBCOMMANDS`
  is unavailable as a token. `verify` collides on the staged asset name:
  `cli_binary_path("accelerator-verify", …)` and `subbinary_asset_path("verify",
  …)` both yield `dist/release/accelerator-verify-<platform>`, so registering it
  would sign the vendored verify shim and advertise it in the manifest. Both
  `verify` and `launcher` additionally shadow real `cli/<name>/` crates through
  `_SUBBINARY_MANIFESTS`' default. A built-in-shadowed token would be signed and
  listed in the manifest but never dispatched. All three constraints are
  enforced by the dispatch guard.

#### 2. The mechanical test

**File**: `tests/unit/tasks/test_registration_docs.py` (new)
**Changes**: Its own file rather than the dispatch guard's, following the
`test_mise.py` / `test_workflows.py` precedent for guards over non-Python
artefacts — the checklist spans Cargo manifests, `.gitignore`, workflow YAML and
user docs, of which only points 1, 7, 10 and 12 concern dispatch.

Read `tasks/README.md`, slice the `## Registering a dispatched sub-binary`
section to the next `## ` heading, and assert:

- exactly **thirteen** numbered items;
- each item body's first line begins, **after stripping leading Markdown
  emphasis**, with a verb from the closed set (`Add`, `Register`, `Update`,
  `Document`, `Extend`) **or** contains `No action when`. The strip is part of
  the contract: every item is written `1. **Add** …`, so a bare
  `startswith(verb)` would match none of the thirteen. `Stage` is dropped from
  the set — point 8 now leads with `Add`, so nothing uses it;
- every item carries exactly one of the `**[PR]**`, `**[release]**` or
  `**[author]**` tags, so the lead-in's promise about where a mistake surfaces
  cannot drift from the items;
- the section contains the literal strings `DISPATCHED_SUBBINARIES`,
  `SKILL_EXEMPT_SUBBINARIES`, `DEBUG_ARCHIVE_DIRS`, `BUILTIN_SUBCOMMANDS`,
  `_SUBBINARY_MANIFESTS`, `_CLI_RELEASE_BINARIES`, `EXPECTED_INJECTION_SKILLS`,
  `package.description`, `version.workspace`, `cli/Cargo.toml`, `bin/<token>-*`,
  `manifest.example.json`, `test_manifest_contract.py`, `resolve/manifest.rs`,
  `_assert_static_elf`, `dist/release/accelerator-*`, `cli/deny.toml`,
  `cli/Cargo.lock`, `accelerator-<token>`, `ACCELERATOR_<TOKEN>_BIN` and `same
  change`;
- **and, for every source the section attributes something to, that the named
  thing still resolves there**: `DISPATCHED_SUBBINARIES`,
  `SKILL_EXEMPT_SUBBINARIES`, `DEBUG_ARCHIVE_DIRS` and `subbinary_asset_path` in
  `tasks/shared/paths.py`; `_SUBBINARY_MANIFESTS` in `tasks/manifest.py`;
  `_assert_static_elf`, `_assert_magic_bytes` and `_CLI_RELEASE_BINARIES` in
  `tasks/build.py`; `prerelease_prepare` and `release_prepare` in
  `tasks/release.py`; `BUILTIN_SUBCOMMANDS` in
  `tasks/shared/dispatch_coherence.py`; `EXPECTED_INJECTION_SKILLS` in
  `tasks/lint/skill_permissions.py`; `Command` and `External` in
  `cli/launcher/src/launch/inbound/cli.rs` — the source point 10 actually
  attributes authority to, where an earlier draft pinned `is_root_help`, which
  the section no longer names and which Phase 2's cross-language test already
  covers; `## Documentation` in the root `README.md`; `[lints.clippy]` in
  `cli/visualiser/server/Cargo.toml`; `ACCELERATOR_` in
  `cli/launcher/src/launch/core.rs`; `**/bin/*.debug.tar.gz` in `.gitignore`;
  `members` in `cli/Cargo.toml`; `build:cli:cross-compile` in `mise.toml`;
  `dist/release/accelerator-*` in `.github/workflows/main.yml`;
  `ACCELERATOR_VISUALISER_BIN` in `docs/visualiser.md`; and
  `test_manifest_contract.py`, `resolve/manifest.rs` and `cli/deny.toml`
  existing as paths.

The second half is what makes the guard bidirectional. A test that reads only
the README freezes doc text: renaming `DISPATCHED_SUBBINARIES` in code would
leave the checklist stale and the test green, while an author *correcting* the
README to the new name would make it fail — a guard that resists the maintenance
it exists to force. Paired, a rename fails the test until both sides move.

The "imperative action line" predicate is not mechanically decidable in general
— a review-1 accepted major, restated in review 2. The closed verb set is a
whitelist coupled to English prose, so an innocuous rewording can fail the build
for a reason that is not a defect. It is retained because an acceptance
criterion requires it; the mitigation is that the set is named in this plan
*and* in the test, and all thirteen items above are drafted against it, so the
section and the predicate cannot be discovered to disagree at implementation
time.

#### 3. Discoverability

**File**: `tasks/CLAUDE.md`
**Changes**: Add a third bullet to the two-bullet file, verbatim: "Registering a
new dispatched sub-binary is a thirteen-point surface — see
`tasks/README.md#registering-a-dispatched-sub-binary` before adding one." The
checklist test also asserts that anchor string appears here, so the pointer
cannot rot while the heading is renamed.

**File**: `CLAUDE.md` (repo root)
**Changes**: The sentence describing `tasks/README.md` as documenting "the
*shape* of the task tree (learn it once)" becomes incomplete once the file also
carries a cross-cutting registration procedure; extend it to name both. Without
these two pointers, discovery depends entirely on already holding the anchor
link — which only 0168 and 0170–0173 carry, not 0169 and not any later author.

### Success Criteria

#### Automated Verification

- [ ] Checklist test passes: `uv run pytest
      tests/unit/tasks/test_registration_docs.py -v`
- [ ] Full unit suite passes: `mise run test:unit:tasks`
- [ ] Read-only CI mirror passes: `mise run check`

#### Manual Verification

- [ ] The GitHub anchor `tasks/README.md#registering-a-dispatched-sub-binary`
      resolves — the four sibling work items (0170–0173) already link to it.
- [ ] The section reads as a checklist an author can follow top-to-bottom
      without consulting the research or this plan, starting from the lead-in
      definition of "token".
- [ ] The new section wraps at 80 columns. There is no markdown formatter or
      linter in this repo — `format:build-system:check` is ruff over Python — so
      width is hand-checked.
- [ ] Renaming `DISPATCHED_SUBBINARIES` in `tasks/shared/paths.py` reddens the
      checklist test, and reverting restores green — the cross-reference half is
      live.

---

## Testing Strategy

### Unit Tests

- **The parsing leaf** — authored, not moved: verified that no direct test of
  any parsing primitive exists today, so the seven functions Phase 1 promotes to
  a shared contract would otherwise arrive uncovered. Includes the `covered_by`
  matcher-contract table, whose path-alias and quoting rows record — rather than
  close — the one evasion both guards share.
- **The guard** — the case matrix in Phase 2, all against fixture tokens
  injected through parameters, every raising case asserting a discriminating
  substring so a pass cannot come from the wrong cause. The non-vacuity cases
  are the ones that prove the guard fails rather than merely passes: missing
  binding, empty collection, ancestor glob, wildcarded segment, wrong-subcommand
  rule, prose-only, second-of-two, bare `Bash` alone, bare `Bash` beside a
  scoped rule, scoped rule beside an ancestor glob, chained command,
  invoked-but-exempt, stale exemption, exemption-equals-collection, reserved
  token, built-in-shadowing token and invalid charset. The cases that must
  *pass* and would have failed the drafted design — a rule scoped tighter than
  the token, a flag-glob rule, a bare `--version`, an `accelerator-verify-*`
  invocation — are what stop the guard reddening releases for correct skills.
- **Source scans** — regex over `read_text`, per the
  `test_bootstrap_coverage.py` house idiom, never `ast`. Five scans: no
  `_BARE_LAUNCHER` under `tasks/`; no `_VISUALISE_SKILL_RELATIVE` under
  `tasks/`; no `visualiser` in `tasks/shared/dispatch_coherence.py`;
  `skill_parsing.py` importing nothing but `re` and `fnmatch`; and the positive
  import assertion with its use-not-just-import, no-shadowing, no-private-import
  and no-own-matcher companions. The last forbids `Bash(`, `fnmatch`, and any
  `re.compile` whose pattern mentions `Bash` or the `!`-preprocessor form — the
  acceptance criterion's wording, which a blanket `re.compile` ban would
  overshoot, since the guard legitimately owns a token-charset regex.
- **Cross-language pins** — `BUILTIN_SUBCOMMANDS` against the clap `Command`
  enum (the dispatch authority) with `is_root_help` as a secondary consistency
  check, per `test_manifest_contract.py`. This converts checklist point 10 from
  prose into an invariant in both the addition and removal directions.
- **Signature pinning** — `inspect.signature` identity assertions on the guard's
  two defaults, plus the source-scan companion that covers the
  interned-empty-tuple degeneracy.
- **Path helpers and registries** — `debug_archive_path` for an injected
  directory; `_debug_archive_targets` for an injected two-token registry
  including both rejection cases; and one literal pin each on
  `DISPATCHED_SUBBINARIES` and `DEBUG_ARCHIVE_DIRS`, so an emptied registry
  fails loudly instead of silently disabling the loops that derive from it.
- **Workflows** — the extended attest test: one attestation per signing step per
  job, `subject-path` set equality across blocks, and coverage derived from
  `_release_uploads()` with a non-empty guard on the derived set.
- **The release call site** — a spy on `validate_dispatch_coherence` through
  `emit_manifest` in `test_manifest.py`, with `sign_file` patched so the test
  needs neither a key nor `minisign` and cannot degrade into a skip. Asserts
  exactly one argument-free call. The three existing `TestEmitManifest` tests
  patch the guard so a skills-tree edit cannot redden them.
- **Docs** — the thirteen-point structural and literal-string assertions, plus
  the source-existence half that makes a rename in code fail the test.
- **Task wiring** — `lint:dispatch-coherence:check` pinned into both
  `build-system:check` and `lint:check`, because `check` does not depend on
  `lint:check` and no CI job runs it.

### Integration Tests

- **The builders** — injected two-token collections and default calls for all
  three, as pure list-derivation assertions, with expected counts derived from
  `len(TARGETS)` rather than hardcoded. No signing key, no network.
- **Manifest agreement** — `_publish` raises `RuntimeError` when the staged
  manifest's `binaries` set differs from `DISPATCHED_SUBBINARIES` in either
  direction, and does not raise when they agree.
- **Ignore semantics** — `pathspec.GitIgnoreSpec` asserts the widened rule
  matches an archive under a nested `bin/` tree, and `TestLeakedArtifactGuard`
  gains a `.debug.tar.gz` case. Both controls were otherwise untested.
- **The lint leaf fails** — `pytest.raises(Exit)` with `violations` patched to
  return one problem, and no raise when it returns `[]`. Without it the PR-time
  gate could be green forever.
- **Regression** — the committed `assert len(uploads) == 22` pin must stay green
  through Phases 3 and 4 (after Phase 3's `_setup_release` fixture update, which
  the phase lists explicitly), and the migrated second-token injection must keep
  exercising a second token through `_release_reverifies`. Together they are the
  guard against the debug-archive generalisation or the unified defaults
  changing real behaviour.

### Manual Testing Steps

1. `rg 'visualiser' tasks/shared/dispatch_coherence.py
   tasks/shared/skill_parsing.py tasks/github.py` returns nothing.
2. `rg '_BARE_LAUNCHER|_VISUALISE_SKILL_RELATIVE' tasks/` returns nothing.
3. `rg 'import' tasks/shared/skill_parsing.py` shows only `fnmatch` and `re`.
4. `jj diff --stat skills/visualisation/visualise/SKILL.md` is empty.
5. Broaden the visualiser's `allowed-tools` rule to
   `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator *)` and confirm `mise run
   build-system:check` reddens naming that SKILL.md; revert. Then narrow it to
   `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser --owner-pid *)` and
   confirm it stays **green** — the two halves of the review-1 probe defect,
   checked against the real tree rather than only against fixtures.
6. Temporarily change one `subject-path` in one of the three
   `attest-build-provenance` blocks and confirm `test_workflows.py` fails;
   delete one whole block and confirm it also fails.
7. Outside this jj workspace, confirm `git check-ignore -v` matches
   `**/bin/*.debug.tar.gz` for a path under a nested `bin/` tree — the parent
   repo's `workspaces/` rule masks the answer from inside.
8. `invoke --help github.upload-and-verify-release` lists only `version`.
9. Grep re-verification of the hand-off edges (below).

## Hand-off Re-verification

Verified green during planning at `d7b55d3`; re-verify at acceptance in case a
sibling was rewritten in the interim:

- `blocked_by: work-item:0187` on 0170, 0171, 0172, 0173 (all at `:12`), and
  0169.
- `blocks: ["work-item:0187"]` on 0168 (`:13`).
- Epic 0136's 0187 annotation
  (`meta/work/0136-migrate-shell-scripts-to-rust-cli.md:65-70`) includes 0172 in
  the unblocks list and names the two 0168 discharge routes.

The blocker is **discharged**: 0168 is `status: done`, closed by commit
`abfaf60bd9`, and it pinned `cli/visualiser/server` by acceptance criterion. The
work item's Dependencies prose ("its work item is still `ready`") is stale by
one day, and the "Visualiser manifest path re-verification" Validation Results
slot is *not required*.

## Performance Considerations

The guard walks `skills/**/SKILL.md` — 45-odd files — once per release and once
per `build-system:check`, replacing a single file read. Negligible against a
release that cross-compiles four targets, and it is the cheapest leaf in that
roll-up. The scan is pure text; no subprocess, no I/O beyond the reads.

## Migration Notes

No data or on-disk format changes. `DEBUG_ARCHIVE_DIRS` and
`SKILL_EXEMPT_SUBBINARIES` are new constants whose initial values reproduce
today's behaviour exactly, so no release in flight is affected. Phase 1 is a
pure move, but it is not consumer-neutral:
`tests/integration/support/skill_corpus.py` re-points at the leaf in the same
change, because `skill_permissions` is deliberately not left as a re-export
façade.

Three behavioural changes, all intended:

- `test_both_absent_is_coherent`: an empty `DISPATCHED_SUBBINARIES` previously
  passed the guard and now fails it.
- The guard is now stricter about what binds. It is verified green against the
  tree at pickup (Phase 2's manual checks), but any skill edit landing between
  drafting and pickup that broadens a launcher rule to an ancestor glob, or
  introduces a launcher invocation from a bare-`Bash` skill, will surface as a
  red `lint:dispatch-coherence:check` on the phase's first run rather than as a
  red release. That is the reason the guard is wired into a CI-reachable roll-up
  in the same phase that introduces it.
- `.gitignore`'s archive rule changes from `bin/*.debug.tar.gz` to
  `**/bin/*.debug.tar.gz`, which is a widening. Verified that the narrow form
  never matched the skill tree the archives are written into, so this ignores
  files that were previously untracked-and-unignored — no tracked file becomes
  ignored, and nothing already committed is affected.

## Deviations from the Work Item

Recorded here so validation does not read them as drift:

- **The parsing is extracted to `tasks/shared/skill_parsing.py`** (Phase 1), and
  the guard imports from there rather than from `tasks.lint.skill_permissions`
  as the Requirements and the reuse acceptance criterion specify. The
  criterion's intent — that the guard reuse the lint module's parsing rather
  than re-implement it — is preserved and still asserted positively; the
  extraction makes the reuse structural instead of an upward dependency from
  `tasks/shared/` into `tasks/lint/`. `BARE_LAUNCHER` is still importable from
  `skill_permissions`, so that criterion's letter holds too. `launcher_token`
  and the `LAUNCHER` prefix live in the leaf as well, so the launcher path has
  one spelling.
- **The guard lives in `tasks/shared/dispatch_coherence.py`**, not
  `tasks/build.py`, and its tests in
  `tests/unit/tasks/shared/test_dispatch_coherence.py`, not `test_build.py`.
  Forced by the circular import; `tasks/shared/` is legitimate once Phase 1 has
  landed, because the module then imports only `tasks.shared.*`.
- **The permission probe is the actual invoked command plus a token-segment
  equality check**, not the bare `${CLAUDE_PLUGIN_ROOT}/bin/accelerator <token>`
  the Requirements specify. That probe is empirically `False` against the
  visualiser's real rule, and the sentinel-argument variant drafted in its place
  is wrong in both directions — see the table in Current State Analysis.
  `BARE_LAUNCHER` moves out of the per-rule matcher, where it would be dead
  code, into the skill-level veto, where it catches globs at or above the binary
  segment that the charset check cannot see.
- **Bare `Bash`, bare-launcher rules and wildcarded token segments are vetoed at
  skill level**, so a skill carrying any of the three cannot witness a binding
  even alongside a correctly scoped rule. Unstated by the item; stated and
  tested here.
- **A chained (metacharacter-bearing) `!` command never binds**, because
  `skill_permissions` refuses to coverage-check one and the two guards must not
  disagree about what a valid invocation is. The **invocation→registration**
  direction is deliberately more permissive than the binding direction: it scans
  every launcher occurrence anywhere in the command text, because a token it
  cannot see is a token that ships unregistered.
- **The guard bounds its own registry** — raising on a stale exemption, on an
  exemption set covering every token, on an exempt token that *is* invoked, on
  `verify`/`launcher`, on a token shadowing a launcher built-in, and on a token
  outside `^[a-z][a-z0-9-]*$`. The item forbids exempting `visualiser` by
  acceptance criterion and states the reserved-name and charset rules in prose,
  but leaves all of them unenforced.
- **The guard also runs as `lint:dispatch-coherence:check`**, wired into
  `build-system:check` *and* `lint:check`, in addition to the argument-free
  release call the item pins. Verified that `check` does not depend on
  `lint:check` and no CI job runs it, so `lint:check` alone would not have
  delivered the PR-time gate.
- **The built-in set includes `help`** alongside `version` and `config`, and is
  pinned against the clap `Command` enum by test rather than by the item's prose
  lockstep.
- **The checklist has thirteen points**, not ten — an eleventh for user-facing
  docs, a twelfth for `DEBUG_ARCHIVE_DIRS` (which Phase 3 creates as a
  registration point), and a thirteenth for `cli/deny.toml`. Amends that
  acceptance criterion.
- **The checklist test also asserts each registration point resolves in
  source**, not only that the README names it — the literal-string half alone
  cannot detect a rename in code.
- **The debug-archive hardwiring is absorbed here** (Phase 3) rather than raised
  as a sibling task, overriding the item's "local to the token loop" boundary
  rule. It carries a `.gitignore` correction and an `_ARTIFACT_MARKERS`
  extension with it, because the `bin/` constraint the registry needs is
  meaningless until the ignore rule actually covers a nested `bin/` tree.
- **Checklist point 9 names all three SLSA blocks**, the workflow gains
  `manifest.json` and `manifest.minisig` subject paths, and the existing attest
  test is extended to assert one attestation per signing step per job plus
  coverage derived from `_release_uploads()`.
- **All four seams use a direct default on a private helper**, and every `@task`
  — `upload_and_verify_release` and `create_debug_archives` alike — stays
  argument-free, because an invoke `@task` parameter becomes an operator-facing
  CLI flag and a `Mapping` default is not expressible on a command line. The
  rule covers token collections only: `RELEASE_MANIFEST`/`RELEASE_MANIFEST_SIG`
  stay module globals read at call time, and `test_github.py`'s three module
  patches stay with them. `collect_entries`' first parameter is renamed to
  `tokens`.
- **The manifest/token set equality lives in `_publish`**, raising
  `RuntimeError` beside `_assert_no_leaked_artifacts` and before
  `commit_version`. Not in `emit_manifest`, where both operands derive from one
  constant so the comparison is a tautology and all three committed
  `TestEmitManifest` tests would redden; and not in `_subbinary_reverifies`,
  which runs outside the draft-preserve envelope and after the tag is pushed.
  `_publish` reads the manifest back from disk, which is where a stale earlier
  cut — reachable, since `*:finalise` is separately invocable — actually shows
  up.
- **`_subbinary_reverifies` keeps `if not names: return []`.** Anti-vacuity on
  the token collection is owned by `_registry_problems` at sign time; a second
  copy in a function reached after the tag is pushed would restate it at the
  worst point in the flow.
- **The `.gitignore` archive rule is widened to `**/bin/*.debug.tar.gz` and
  `_assert_no_leaked_artifacts` gains `-uall`.** Verified the narrow form never
  matched the tree the archives are written into, and that porcelain's default
  untracked mode collapses a wholly-untracked directory to one line, which would
  hide the new `.debug.tar.gz` marker in exactly the scenario it is written for.
- **The Test-seams fallback is not taken** — the signing extraction is local.

## References

- Original work item:
  `meta/work/0187-generalise-sub-binary-registration-surface.md`
- Related research:
  `meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`
- Work item review:
  `meta/reviews/work/0187-generalise-sub-binary-registration-surface-review-1.md`
- Plan review:
  `meta/reviews/plans/2026-08-02-0187-generalise-sub-binary-registration-surface-review-1.md`
- Parent epic: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Governing decision:
  `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
- Blocker: `meta/work/0168-fold-visualiser-into-cli-workspace.md` (**done**)
- Parsing contract reused: `tasks/lint/skill_permissions.py:106-113`
  (`covered_by`), moved to `tasks/shared/skill_parsing.py` by Phase 1
- Parameter-with-default precedent: `tasks/manifest.py:69` (`collect_entries`)
- Source-scan idiom: `tests/unit/tasks/test_bootstrap_coverage.py:54-74`
- Workflow-yaml idiom: `tests/unit/tasks/test_workflows.py`, extended at
  `:147-159`
- Rust-literal pinning idiom: `tests/unit/tasks/test_manifest_contract.py:51-74`
- Pure-helper-plus-thin-task idiom: `tasks/build.py`
  (`vendor_shim_marker_digest` / `vendor_verify_shims`)
- Lint-task registration pattern: `tasks/README.md:32-38`
