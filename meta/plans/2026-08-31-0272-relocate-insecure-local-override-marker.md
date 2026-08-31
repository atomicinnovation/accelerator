---
type: "plan"
id: "2026-08-31-0272-relocate-insecure-local-override-marker"
title: "Relocate Insecure-Local Override Marker to .accelerator Implementation Plan"
date: "2026-08-31T20:42:23+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "done"
work_item_id: "work-item:0272"
parent: "work-item:0272"
derived_from: ["codebase-research:2026-08-31-0272-relocate-insecure-local-override-marker"]
tags: ["security", "config", "cleanup"]
revision: "793f8dbcd3bc0fb8b4cd151c76002757720c9b22"
repository: "accelerator"
last_updated: "2026-08-31T22:09:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Relocate Insecure-Local Override Marker to .accelerator Implementation Plan

## Overview

Move the credential-override marker from `.claude/insecure-local-ok` to
`.accelerator/allow-insecure-local`, consolidating every Accelerator-owned file
under `.accelerator/` and aligning the marker's basename with the
`ACCELERATOR_ALLOW_INSECURE_LOCAL` environment variable it pairs with. The move
is also the moment to retire the duplicated production path literal: a new
`INSECURE_MARKER_RELATIVE` constant in `tracker-support` becomes the single
source of truth that the three production builders and the two contract harnesses
reference, so the compiler — not a grep — guarantees no caller drifts to a stale
path. The change also corrects the Jira documentation so it accurately describes
the shared refuse-plus-override behaviour and genuinely mirrors the Linear prose.
The resolver logic is untouched. Hard cutover — the old path is dropped, not read
as a fallback.

## Current State Analysis

The override is a four-gate check on the personal config file
(`cli/tracker-support/src/credentials.rs:418-428`): a non-`0600`
`config.local.md` is admitted only when `ACCELERATOR_ALLOW_INSECURE_LOCAL` is
exactly `"1"` **and** the marker is a regular, non-symlink, VCS-tracked file.
The resolver reads the marker abstractly through the
`CredentialContext.insecure_marker: PathBuf` field (`credentials.rs:141`,
consumed at `:424`); it never constructs the path literal. Each caller supplies
its own `PathBuf`, so the marker path is duplicated across three production
context builders and five test fixtures — never shared.

Every other Accelerator-owned file already lives under `.accelerator/`
(`config.md`, `config.local.md`); the marker is the sole outlier at `.claude/`.
The insecure-local functionality is unreleased, so no repository has committed a
marker at the old path and the hard cutover strands nothing.

## Desired End State

A single `INSECURE_MARKER_RELATIVE` constant in `tracker-support` holds the
production path; the three production builders and the two contract harnesses
reference it, the resolver docstring and both `SKILL.md` override passages name
`.accelerator/allow-insecure-local`, and the three flat test fixtures name the
bare basename `allow-insecure-local`. The Jira section describes the same
refuse-plus-override behaviour as the Linear section — its inaccurate "warns if
looser than `0600`" wording is corrected to "refuses", matching the shared
resolver. The override's security semantics are byte-for-byte identical — only
the supplied path moves. Verified when `mise run check` exits 0, the new seam and
symlink tests pass, and the acceptance-criteria greps hold: no `insecure-local-ok`
and no `.claude/allow-insecure-local` remain under `cli/`, `skills/`, or `hooks/`;
the constant carries `.accelerator/allow-insecure-local`; the three builders and
two harnesses reference `INSECURE_MARKER_RELATIVE`; and the resolver docstring and
both `SKILL.md` sections name the new path.

### Key Discoveries:

- **The path is a caller concern; the check is a resolver concern.** The
  `insecure_marker: PathBuf` field is the seam — `refuse_insecure_personal_config`
  (`credentials.rs:378-405`) and `insecure_override_allowed` (`:418-428`) are
  path-agnostic, so this is a caller-side edit plus a docstring.
- **The fixtures use two literal forms.** Three flat fixtures join a bare
  `insecure-local-ok` onto a temp root whose `config.local.md` is also a bare
  sibling (`credentials.rs:139`/`:135`, `jira-client/tests/support/mod.rs:135`,
  `linear-client/tests/support/mod.rs:140`). Two contract harnesses join
  `.claude/insecure-local-ok` against a real repo root whose personal config is
  `.accelerator/config.local.md` (`jira-client/tests/contract.rs:105`,
  `linear-client/tests/contract.rs:124`).
- **One fixture writes a real marker file.** `credentials.rs:409`
  (`fs::write(&marker, "")`) inside
  `the_insecure_override_needs_both_the_variable_and_a_tracked_marker`. Keeping
  this flat fixture's basename bare leaves the write's parent as the temp root —
  no `create_dir_all` is needed.
- **No error string names the marker.** `E_LOCAL_PERMS_INSECURE`
  (`credentials.rs:189-194`) interpolates only the config path and octal mode, so
  the runtime message needs no edit.
- **The public-API snapshot is unaffected.** It pins the field name
  `insecure_marker` (`cli/tracker-support/tests/fixtures/public-api.txt:69,275`),
  which does not change — no `cargo-public-api` regeneration.
- **The Jira doc prose is inaccurate and asymmetric.** The Linear section
  documents refuse-plus-override and claims to mirror Jira (`SKILL.md:808-811`),
  but the Jira section says the resolver "warns if looser than `0600`"
  (`SKILL.md:708-709`) and names no override. The shared, path-agnostic resolver
  refuses for both integrations, so "warns" is wrong and the mirror claim is
  unbacked; the symmetric fix corrects both.

## What We're NOT Doing

- **No fallback and no migration aid.** The old path is not read as a
  secondary source; no migration reminder is added. The feature is unreleased.
- **No change to the override semantics.** The env-var, regular-file,
  non-symlink, and VCS-tracked gates stay identical; only the path moves.
- **No de-duplication of the sibling `personal_config` path.** The refactor is
  limited to the marker path — the security scope of work item 0272. The
  symmetric `.accelerator/config.local.md` literal stays duplicated across the
  three builders and could likewise be routed through the existing
  `Level::Personal.filename()` constant, but that is deferred to keep this change
  scoped to the marker relocation; a full shared `CredentialContext` builder is
  also out of scope.
- **No `E_LOCAL_PERMS_INSECURE` message change** — it never names the marker.
- **No broader `SKILL.md` rewrite.** The Jira edit is scoped to the
  permissions/override sentence; the surrounding token-resolution prose,
  examples, and key tables are left as they are.

## Implementation Approach

Single atomic change. Splitting the constant extraction, the rename, and the doc
correction into phases would be artificial: runtime behaviour is unchanged and
the pieces share one commit's worth of coordinated edits.

Test-driven development applies in two shapes here. The constant is a
change-detector, not a red-green cycle: `INSECURE_MARKER_RELATIVE` does not exist
before the extraction, so the pin test does not compile until the constant is
added, then locks its value. Because the three builders and two contract
harnesses reference the constant rather than a literal, the compiler — not a grep
— guarantees they cannot drift to a stale path, so acceptance criterion #2 (the
legacy path is no longer honoured) becomes a compile-time property backed by one
durable test, not a one-shot grep.

The symlink cases are characterisation tests. The resolver's symlink gates
(`credentials.rs:388-393` for the personal config, `:426` for the marker) already
exist but are untested — the plan's earlier claim that they were covered was
wrong — so the new tests pass immediately and close a standing coverage gap
rather than driving new behaviour. The override, tracked, and untracked cases
remain covered by the existing path-agnostic tests, which keep passing after the
fixture path moves.

## Phase 1: Coordinated marker rename

### Overview

Extract the production path into a shared `INSECURE_MARKER_RELATIVE` constant and
point the three production builders and two contract harnesses at it; rename the
three flat test-fixture basenames; update the resolver docstring; add the seam
pin and two symlink tests; and correct both `SKILL.md` passages so the Jira
section describes the shared refuse-plus-override behaviour symmetrically with
Linear. Run `mise run check` and the acceptance-criteria greps.

### Changes Required:

#### 1. Shared marker-path constant

**File**: `cli/tracker-support/src/credentials.rs` (plus the crate-root
re-export in `cli/tracker-support/src/lib.rs`)
**Changes**: Define the production marker path once, near `CredentialContext`,
and re-export it at the crate root alongside `CredentialContext` so callers reach
it as `tracker_support::INSECURE_MARKER_RELATIVE` (matching the existing
re-export convention).

```rust
/// Repo-relative path of the insecure-local override marker.
pub const INSECURE_MARKER_RELATIVE: &str = ".accelerator/allow-insecure-local";
```

The one-line rustdoc matches every other top-level `pub` item in
`credentials.rs`; it documents public API rather than restating the value, so it
sits within the repo's comment convention.

#### 2. Production context builders

**Files**: `cli/jira-cli/src/context.rs`, `cli/linear-cli/src/context.rs`,
`cli/work-cli/src/tracker_registry.rs`
**Changes**: Repoint each `insecure_marker` join to reference the constant
instead of a literal, and add the `use` alongside the existing
`use tracker_support::CredentialContext;`.

```diff
-        insecure_marker: root.join(".claude/insecure-local-ok"),
+        insecure_marker: root.join(INSECURE_MARKER_RELATIVE),
```

#### 3. Resolver docstring

**File**: `cli/tracker-support/src/credentials.rs`
**Changes**: Name the new path in the `refuse_insecure_personal_config`
docstring (`:375-377`).

```diff
 /// The mode-0600 gate on the personal config file, with an override:
 /// `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` counts only when
-/// `.claude/insecure-local-ok` is a regular, non-symlink, VCS-tracked file.
+/// `.accelerator/allow-insecure-local` is a regular, non-symlink, VCS-tracked
+/// file.
```

#### 4. Flat test fixtures — bare basename

**Files**: `cli/jira-client/tests/support/mod.rs` (`:135`),
`cli/linear-client/tests/support/mod.rs` (`:140`)
**Changes**: Rename the bare basename, keeping it a sibling of the fixture's
bare `config.local.md`. The parent stays the temp root, so the write site in
`credentials.rs` needs no `create_dir_all`. These flat fixtures keep a bare
basename rather than referencing the constant because the constant's
`.accelerator/` prefix would force a `create_dir_all` at the one write site (the
`the_insecure_override_needs_both_the_variable_and_a_tracked_marker` write, see
the Testing Strategy "The one write site" bullet). This rationale stays in the
plan; do not embed it as a code comment in the fixture.

```diff
-        insecure_marker: root.join("insecure-local-ok"),
+        insecure_marker: root.join("allow-insecure-local"),
```

For the `Workspace::marker` helper in `cli/tracker-support/tests/credentials.rs`
(`:138`):

```diff
     fn marker(&self) -> PathBuf {
-        self.root.path().join("insecure-local-ok")
+        self.root.path().join("allow-insecure-local")
     }
```

#### 5. Contract-harness fixtures — reference the constant

**Files**: `cli/jira-client/tests/contract.rs` (`:105`),
`cli/linear-client/tests/contract.rs` (`:124`)
**Changes**: These harnesses join the full path against a real repo root, so they
reference the same constant as the production builders rather than a literal.

```diff
-        insecure_marker: root.join(".claude/insecure-local-ok"),
+        insecure_marker: root.join(tracker_support::INSECURE_MARKER_RELATIVE),
```

#### 6. New tests — constant value and symlink gates

**File**: `cli/tracker-support/tests/credentials.rs`
**Changes**: Add three tests. The first pins the seam; the other two are
Unix-gated characterisation tests filling a real coverage gap (the resolver
already implements these gates but nothing exercises them).

- `the_marker_path_lives_under_accelerator` — asserts
  `INSECURE_MARKER_RELATIVE == ".accelerator/allow-insecure-local"`. This is a
  change-detector on the constant's value: it does not compile until the constant
  exists, and thereafter pins the string; the compile-time references in §§2 and 5
  carry that guarantee to every caller.
- A Unix-gated test isolating the personal-config symlink gate
  (`credentials.rs:388-393`). Set every other condition so the gate alone
  decides the outcome: `ACCELERATOR_ALLOW_INSECURE_LOCAL=1`, a tracked marker
  present, and the symlink pointing at a valid personal config — so that were the
  `:388` symlink check removed, the override would admit the read. With the gate
  in place the resolver refuses with `E_LOCAL_PERMS_INSECURE`.
- A Unix-gated test isolating the marker `is_file()` gate
  (`credentials.rs:426`). Set `ACCELERATOR_ALLOW_INSECURE_LOCAL=1`, a regular,
  non-symlink, non-`0600` personal config, and a **tracked** marker that is a
  symlink to a real regular file — so only `is_file()` returning false for the
  symlink causes the refusal; were that gate relaxed to `is_tracked` alone, the
  override would admit the read. With the gate in place the resolver refuses with
  `E_LOCAL_PERMS_INSECURE`.

Both symlink tests follow the existing `#[cfg(unix)]` split used by `set_mode` in
this file, creating links with `std::os::unix::fs::symlink`. Pinning the other
gates this way keeps each test a durable guard for its own gate — a resolver
mutation that deletes the gate flips the test red, rather than the test passing
for an unrelated reason.

#### 7. User-facing documentation — Linear passage

**File**: `skills/config/configure/SKILL.md` (`:808-811`)
**Changes**: Name the new marker path in the Linear personal-settings override
note. The "mirroring the Jira integration" cross-reference stays and becomes
truthful once the Jira passage below is corrected.

```diff
-`ACCELERATOR_ALLOW_INSECURE_LOCAL=1` plus a committed `.claude/insecure-local-ok`
-marker), mirroring the Jira integration.
+`ACCELERATOR_ALLOW_INSECURE_LOCAL=1` plus a committed
+`.accelerator/allow-insecure-local` marker), mirroring the Jira integration.
```

#### 8. User-facing documentation — Jira passage (symmetric fix)

**File**: `skills/config/configure/SKILL.md` (`:707-709`)
**Changes**: Replace the inaccurate warn-only sentence with the same
refuse-plus-override description the Linear section carries. The shared resolver
refuses a non-`0600` personal config for Jira exactly as for Linear, so "warns"
is wrong; the override is the same env-var-plus-marker escape hatch.

```diff
 `token` plaintext is supported but discouraged — prefer `token_cmd` with a
-password manager. The resolver checks `config.local.md` permissions and
-warns if looser than `0600`.
+password manager. The resolver refuses to read credentials from a
+`config.local.md` looser than `0600` (override with
+`ACCELERATOR_ALLOW_INSECURE_LOCAL=1` plus a committed
+`.accelerator/allow-insecure-local` marker).
```

### Success Criteria:

#### Automated Verification:

- [x] No legacy literal and no wrong-directory variant remains in code, docs, or
  hooks: both `rg -n "insecure-local-ok" cli/ skills/ hooks/` and
  `rg -n "\.claude/allow-insecure-local" cli/ skills/ hooks/` return no matches.
- [x] The constant is the single source of truth carrying the new path:
  `rg -n 'INSECURE_MARKER_RELATIVE.*"\.accelerator/allow-insecure-local"'
  cli/tracker-support/src/credentials.rs` matches.
- [x] Every production caller references the constant, not a literal:
  `rg -n "INSECURE_MARKER_RELATIVE" cli/jira-cli/src/context.rs
  cli/linear-cli/src/context.rs cli/work-cli/src/tracker_registry.rs
  cli/jira-client/tests/contract.rs cli/linear-client/tests/contract.rs` matches
  all five.
- [x] The docs name the new path: `rg -c "allow-insecure-local"
  skills/config/configure/SKILL.md` returns `2`, and
  `rg -n "allow-insecure-local" cli/tracker-support/src/credentials.rs` matches
  both the docstring and the constant.
- [x] The inaccurate Jira wording is gone:
  `rg -n "warns if looser than" skills/config/configure/SKILL.md` returns no
  matches.
- [x] The marker path is not ignored by this repo's own `.gitignore`:
  `git check-ignore .accelerator/allow-insecure-local` exits with status exactly
  `1` (matched-nothing; treat `128` as failure, since an environment error is not
  a pass). This guards the accelerator repo's dev checkout against a future broad
  `.accelerator/` ignore rule. It does **not** verify an end-user repo's ignore
  rules — the VCS-tracked gate runs against whatever repo the command executes in,
  which this check cannot inspect.
- [x] The `tracker-support` credential tests pass, including the new seam pin and
  the two symlink characterisation tests:
  `cargo test -p tracker-support --test credentials`.
- [x] The Rust workspace is clean: `mise run cli:check`.
- [x] The full read-only CI mirror passes: `mise run check`.

#### Manual Verification:

- [ ] With `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` and a committed, non-symlink
  `.accelerator/allow-insecure-local` in a repo whose `config.local.md` is
  non-`0600`, a tracker command honours the override and reads the credential.
- [ ] With the same env var but the marker only at the legacy
  `.claude/insecure-local-ok`, the command refuses with
  `E_LOCAL_PERMS_INSECURE`.
- [ ] With the marker at the new path but symlinked, or present-but-untracked,
  the command still refuses with `E_LOCAL_PERMS_INSECURE`.
- [ ] Read the Jira and Linear override passages in `SKILL.md` side by side:
  both describe the same refuse-plus-override behaviour and name
  `.accelerator/allow-insecure-local`, so the Linear "mirroring the Jira
  integration" claim now holds.

---

## Testing Strategy

### Unit Tests:

- **New — the seam pin.** `the_marker_path_lives_under_accelerator` asserts
  `INSECURE_MARKER_RELATIVE == ".accelerator/allow-insecure-local"`. This is the
  one red-green test: it fails before the constant carries the new value. Because
  every production caller references the constant, the compiler carries the
  guarantee to all five sites — this test, not the acceptance grep, is the durable
  guard for acceptance criterion #2.
- **New — the two symlink gates.** Add Unix-gated characterisation tests for a
  symlinked personal config (refused at `credentials.rs:388-393`) and a symlinked
  marker (rejected by the `is_file()` gate at `:426`), both asserting
  `E_LOCAL_PERMS_INSECURE`. These gates already exist in the resolver but were
  untested; the plan's earlier "already covered" claim was inaccurate. They pass
  immediately and close the gap.
- **Existing path-agnostic coverage.**
  `cli/tracker-support/tests/credentials.rs` already exercises the override with a
  tracked marker, an untracked marker, and a non-`0600` refusal — all
  path-agnostic. After the fixture basename moves, these assert the same
  behaviour at the new path.
- **The one write site.** Confirm
  `the_insecure_override_needs_both_the_variable_and_a_tracked_marker`
  (`credentials.rs:404-431`) still passes: its `fs::write(&marker, "")` writes to
  the temp root under the bare basename, so no directory creation is required.

### Integration Tests:

- The two live contract harnesses (`jira-client`/`linear-client`
  `tests/contract.rs`) are gated on real tenant credentials and are not part of
  the default run; the edit is a literal swap verified by compilation under
  `mise run check`.

### Manual Testing Steps:

1. In a scratch git repo, write `.accelerator/config.local.md` with a
   `jira.token`, `chmod 0644` it, commit `.accelerator/allow-insecure-local`,
   export `ACCELERATOR_ALLOW_INSECURE_LOCAL=1`, and confirm a tracker command
   reads the credential.
2. Move the marker back to `.claude/insecure-local-ok`, commit it, and confirm
   the command now refuses with `E_LOCAL_PERMS_INSECURE`.
3. Replace the new-path marker with a symlink, then with an uncommitted file,
   and confirm each refuses.

## Performance Considerations

None. The change is a path-literal rename with no effect on the resolver's work.

## Migration Notes

None. The functionality is unreleased, so no committed marker exists at the old
path in any repository. The hard cutover is safe precisely while this holds — it
must land before the insecure-local override feature is released, after which a
committed `.claude/insecure-local-ok` would make the rename a breaking migration.

**Precondition — confirmed intended.** The rename presupposes the insecure-local
override ships as a real feature; it does. The research's "plan-vs-code drift"
concern was a misreading: the 0197 plan's "no bypass gate" statement
(`meta/plans/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md:265-266`)
is scoped to the config-*write* path — `WriteConfigLevel::write` clamps a personal
write to 0600, so `config set` needs no bypass — not the read-side resolver
override. The override was designed deliberately in the origin plan
(`meta/plans/2026-04-29-jira-integration-phase-1-foundation.md:976-996`, the
opt-out with its six-case matrix) and ships wired into the resolver
(`insecure_override_allowed`, `credentials.rs:418-428`) with a matching named
test (`the_insecure_override_needs_both_the_variable_and_a_tracked_marker`). The
hard cutover rests on a verified premise.

## References

- Original work item: `meta/work/0272-relocate-insecure-local-override-marker.md`
- Related research:
  `meta/research/codebase/2026-08-31-0272-relocate-insecure-local-override-marker.md`
- Resolver seam: `cli/tracker-support/src/credentials.rs:135-143,378-428`
- Consolidation rationale:
  `meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md`
