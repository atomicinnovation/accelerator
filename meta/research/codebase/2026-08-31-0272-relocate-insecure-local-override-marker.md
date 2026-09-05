---
type: "codebase-research"
id: "2026-08-31-0272-relocate-insecure-local-override-marker"
title: "Research: Relocate insecure-local override marker to .accelerator"
date: "2026-08-31T20:33:45+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0272"
parent: "work-item:0272"
topic: "Relocate insecure-local override marker to .accelerator"
tags: ["research", "codebase", "credentials", "tracker-support", "security", "config"]
revision: "39dd51d48f99dce20fd5c31a451dfcf52d58a878"
repository: "accelerator"
last_updated: "2026-08-31T20:33:45+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Relocate insecure-local override marker to .accelerator

**Date**: 2026-08-31T20:33:45+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 39dd51d48f99dce20fd5c31a451dfcf52d58a878
**Branch**: HEAD (detached; jj-colocated)
**Repository**: accelerator

## Research Question

For work item 0272 — moving the credential-override marker from
`.claude/insecure-local-ok` to `.accelerator/allow-insecure-local` — where does
the marker path get constructed, how does the resolver consume it, which tests
and docs hard-code the old path, and what security semantics must survive the
move?

## Summary

The relocation is a **path-literal change at seven live sites plus one doc
line**, with the resolver itself untouched. The marker path is duplicated —
never shared — across three production context builders and five test fixtures;
the resolver `refuse_insecure_personal_config` reads it abstractly through the
`CredentialContext.insecure_marker: PathBuf` field, so its logic and the
`E_LOCAL_PERMS_INSECURE` error string need no change. Two things bite: the test
fixtures use **two different literal forms** (bare `insecure-local-ok` vs full
`.claude/insecure-local-ok`), and one fixture **writes a real file** at the
marker path, so relocating it under a `.accelerator/` subdirectory needs the
parent directory created first.

Concrete edit surface:

| Site | Path | Current literal | Note |
|---|---|---|---|
| Production builder | `cli/jira-cli/src/context.rs:165` | `.claude/insecure-local-ok` | 🔵 live |
| Production builder | `cli/linear-cli/src/context.rs:145` | `.claude/insecure-local-ok` | 🔵 live |
| Production builder | `cli/work-cli/src/tracker_registry.rs:124` | `.claude/insecure-local-ok` | 🔵 live |
| Resolver docstring | `cli/tracker-support/src/credentials.rs:377` | `.claude/insecure-local-ok` | 🟣 doc |
| Fixture (writes file) | `cli/tracker-support/tests/credentials.rs:139` | `insecure-local-ok` | 🟠 test |
| Fixture (path only) | `cli/jira-client/tests/support/mod.rs:135` | `insecure-local-ok` | 🟠 test |
| Fixture (path only) | `cli/linear-client/tests/support/mod.rs:140` | `insecure-local-ok` | 🟠 test |
| Fixture (path only) | `cli/jira-client/tests/contract.rs:105` | `.claude/insecure-local-ok` | 🟠 test |
| Fixture (path only) | `cli/linear-client/tests/contract.rs:124` | `.claude/insecure-local-ok` | 🟠 test |
| User doc | `skills/config/configure/SKILL.md:810` | `.claude/insecure-local-ok` | 🟣 doc |

Legend: 🔵 production code · 🟣 documentation prose · 🟠 test fixture.

`hooks/` and `bin/` reference the marker **nowhere**. The public-API snapshot
pins the field name `insecure_marker` (`cli/tracker-support/tests/fixtures/public-api.txt:69,275`),
which does not change, so no `cargo-public-api` regeneration is required.

## Detailed Findings

### The resolver — reads the marker abstractly

`refuse_insecure_personal_config` gates on `context.personal_config` and only
consults the marker through the `context.insecure_marker` field; it never
builds the path literal. The path string appears in this file **only** in the
docstring at `cli/tracker-support/src/credentials.rs:377`. The gate order:

```text
symlink_metadata(personal_config)          # lstat, no deref (382)
  → is_symlink?  → refuse LocalPermsInsecure, mode 0 (388-393)
  → mode & low-6-bits == 0? → accept (394-397)
  → insecure_override_allowed(context)? → accept (398-400)
  → else refuse LocalPermsInsecure, mode (401-404)
```

`insecure_override_allowed` (`credentials.rs:418-428`) ANDs four gates,
short-circuiting left to right:

- **Env var first.** `ACCELERATOR_ALLOW_INSECURE_LOCAL` must equal exactly `"1"`; any other value or absence returns `false` before the marker is touched (`419-422`).
- **Marker present.** `symlink_metadata(marker)` must succeed — lstat again, so a symlinked marker is not dereferenced (`425`).
- **Regular non-symlink file.** `facts.file_type().is_file()` fails a symlink, directory, or socket (`426`).
- **VCS-tracked.** `context.provenance.is_tracked(marker)` must be true (`426`).

The `CredentialContext` struct carrying the field is defined at
`credentials.rs:135-143` (`pub insecure_marker: PathBuf` at `:141`). These are
exactly the semantics the work item requires to stay identical — only the
`PathBuf` supplied by callers changes.

### The three production builders — duplicated, not shared

Each builder hand-writes a `CredentialContext { ... }` literal and assigns
`insecure_marker: root.join(".claude/insecure-local-ok")`. There is **no shared
helper**. In every case the line directly above assigns
`personal_config: root.join(".accelerator/config.local.md")` — the exact
`.accelerator/` pattern the marker should align to.

- `cli/jira-cli/src/context.rs:165` — inside `build_client()`; `root = FileConfigStore::discover_root(&start)`, `start = current_dir()`.
- `cli/linear-cli/src/context.rs:145` — inside `build_client()`; identical `root` derivation.
- `cli/work-cli/src/tracker_registry.rs:124` — inside the free function `credential_context(root: &Path)`, called from `ConfiguredTrackers::resolve` with `&self.root`.

The `.accelerator/config.local.md` sibling at `:164` / `:144` / `:123`
respectively is the reference for what a well-formed `.accelerator/` join looks
like in each file.

### Test fixtures — two literal forms, one real write

Five fixtures construct the marker across the three client crates, in two
shapes:

- **Bare basename `insecure-local-ok`** joined onto a standalone temp root, in the reusable support helpers: `cli/tracker-support/tests/credentials.rs:139` (via `Workspace::marker()`, wired at `:161`), `cli/jira-client/tests/support/mod.rs:135`, `cli/linear-client/tests/support/mod.rs:140`.
- **Full `.claude/insecure-local-ok`** in the live contract harnesses: `cli/jira-client/tests/contract.rs:105`, `cli/linear-client/tests/contract.rs:124`.

⚠️ **One fixture writes a real file.** In
`cli/tracker-support/tests/credentials.rs`, the test
`the_insecure_override_needs_both_the_variable_and_a_tracked_marker` (`:405`)
does `std::fs::write(&marker, "")` at `:408-409` then tracks it via
`FixedProvenance::tracking(&marker)` at `:424`. The current basename sits
directly in the temp root, so the write needs no parent directory. If the
fixture is relocated to `.accelerator/allow-insecure-local`, the `.accelerator`
parent must be created (`fs::create_dir_all`) before the write, or the test
fails at the write call. The other four fixtures construct a path only — never
written, never symlinked — so a literal swap suffices.

❓ **Basename decision for the standalone fixtures.** The three support helpers
join a bare basename onto a fake temp root that does not mirror the real repo
layout. The planner must decide whether they become bare `allow-insecure-local`
(minimal change, write-site keeps no parent dir) or the full
`.accelerator/allow-insecure-local` (mirrors production, write-site needs the
parent dir). The acceptance-criteria grep only forbids `insecure-local-ok`
across `cli/`, `skills/`, `hooks/` — it does not mandate a `.accelerator/`
prefix inside test roots.

### The error message and env-scrubbing tests — no change needed

`E_LOCAL_PERMS_INSECURE` (`credentials.rs:189-192`) interpolates only the
personal-config path and its octal mode, never the marker path, matching the
work item's Technical Notes. The `work-cli` env-scrubbing tests reference the
env var by name (`cli/work-cli/tests/cli_sync.rs:158`,
`cli/work-cli/tests/common/mod.rs:28`) but assert scrubbing behaviour, not the
marker path — they are untouched by the relocation.

### The user-facing doc — one line

Exactly one passage names the marker: `skills/config/configure/SKILL.md:810`,
in the Linear section under `#### Personal settings (do not commit)`. It reads
"…override with `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` plus a committed
`.claude/insecure-local-ok` marker…mirroring the Jira integration." The doc
does **not** spell out the regular-file / non-symlink / VCS-tracked mechanics,
so only this line needs editing.

⚠️ **Pre-existing doc inconsistency.** The Jira section it claims to mirror
(`SKILL.md:708-709`) is warn-only — it mentions no override, no marker, no env
var. Line 811's "mirroring the Jira integration" is not backed by matching Jira
prose. Out of scope for 0272, but worth noting to whoever edits the line.

## Code References

- `cli/tracker-support/src/credentials.rs:135-143` — `CredentialContext` struct; `insecure_marker: PathBuf` at `:141`.
- `cli/tracker-support/src/credentials.rs:377` — resolver docstring naming `.claude/insecure-local-ok` (only in-file occurrence of the literal).
- `cli/tracker-support/src/credentials.rs:378-405` — `refuse_insecure_personal_config` gate.
- `cli/tracker-support/src/credentials.rs:418-428` — `insecure_override_allowed`; env-var-then-marker four-gate check.
- `cli/tracker-support/src/credentials.rs:189-192` — `E_LOCAL_PERMS_INSECURE` message (names config path only).
- `cli/jira-cli/src/context.rs:165` — production marker construction.
- `cli/linear-cli/src/context.rs:145` — production marker construction.
- `cli/work-cli/src/tracker_registry.rs:124` — production marker construction.
- `cli/tracker-support/tests/credentials.rs:139,161,408-409,424` — marker helper + the one real-file write site.
- `cli/jira-client/tests/support/mod.rs:135` / `cli/linear-client/tests/support/mod.rs:140` — bare-basename path-only fixtures.
- `cli/jira-client/tests/contract.rs:105` / `cli/linear-client/tests/contract.rs:124` — full-path path-only fixtures.
- `cli/tracker-support/tests/fixtures/public-api.txt:69,275` — pins `insecure_marker` field name (unchanged by the move).
- `skills/config/configure/SKILL.md:810` — the single user-facing marker reference.

## Architecture Insights

- **The path is a caller concern, the check is a resolver concern.** The
  `CredentialContext.insecure_marker: PathBuf` field is the seam: the resolver
  is path-agnostic, so relocating the marker is a caller-side edit. This is why
  the work item scopes the resolver to a docstring-only change.
- **Duplication is the cost.** The same literal lives at three production sites
  and five test sites because each builds its `CredentialContext` by hand. No
  refactor is in scope, but the duplication is the reason the grep-based
  acceptance criteria enumerate all three builders explicitly.
- **The VCS-tracked gate is the security hinge.** `is_file() && is_tracked`
  requires the marker to be committed, making the override a reviewable act
  rather than something a stray local file can trigger. `.accelerator/.gitignore`
  ignores only `config.local.md`, so `.accelerator/allow-insecure-local` remains
  trackable — the new path satisfies the gate.

## Historical Context

- `meta/notes/2026-06-23-further-ideas-backlog.md:64` — origin of 0272: "Move `insecure-local-ok` marker file under `.accelerator`".
- `meta/reviews/work/0272-relocate-insecure-local-override-marker-review-1.md` — review of the work item: hard-cutover safety (feature unreleased), the grep ACs, the basename rename.
- `meta/plans/2026-04-29-jira-integration-phase-1-foundation.md` — canonical origin design of the whole scheme (bash): `E_LOCAL_PERMS_INSECURE` exit 29, the 0600 fail-closed refusal, symlink rejection, and the dual-gate opt-out with a six-case test matrix.
- `meta/plans/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md` — the Rust port; records that personal writes are clamped to 0600. ❓ It states the port has "no bypass gate", yet the live resolver clearly implements `insecure_override_allowed` — likely plan-vs-code drift where the override was reinstated after that plan; worth confirming the override is intended before release.
- `meta/decisions/ADR-0047-multi-level-userspace-configuration-model.md` — establishes `.accelerator/` with `config.md` + `config.local.md`; the layout 0272 extends to the marker.
- `meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md` — the consolidation rationale 0272 completes.

## Related Research

- `meta/research/codebase/2026-06-14-0048-linear-integration-apis.md:177-178` — prior mention of the env var + `.claude/insecure-local-ok` marker.
- `meta/research/codebase/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md:214-215` — the error code + VCS-tracked-marker override in the port research.

## Open Questions

- ❓ **Standalone-fixture basename.** Bare `allow-insecure-local` or full `.accelerator/allow-insecure-local` inside the temp-root test helpers? The latter forces `create_dir_all` at the one write site (`credentials.rs:408-409`).
- ❓ **Override intent vs. the 0197 plan.** The port plan says "no bypass gate"; the code has one. Confirm the override is deliberate before the feature releases, since 0272's hard cutover assumes the override ships.
- ❓ **Jira doc parity.** `SKILL.md:811` claims Linear "mirrors the Jira integration", but the Jira prose (`:708-709`) is warn-only. Fix in 0272 or leave for a follow-up?
