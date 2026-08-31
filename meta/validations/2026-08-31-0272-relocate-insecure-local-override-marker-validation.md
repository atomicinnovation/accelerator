---
type: "plan-validation"
id: "2026-08-31-0272-relocate-insecure-local-override-marker-validation"
title: "Validation Report: Relocate Insecure-Local Override Marker to .accelerator"
date: "2026-08-31T22:56:00+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "plan:2026-08-31-0272-relocate-insecure-local-override-marker"
target: "plan:2026-08-31-0272-relocate-insecure-local-override-marker"
tags: ["security", "config", "cleanup"]
last_updated: "2026-08-31T22:56:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Relocate Insecure-Local Override Marker to .accelerator

Result: **pass**. The single-phase plan is fully implemented in commit
`30fa84921f`. All eight automated acceptance greps hold, the `tracker-support`
credential suite passes (22 tests, including the seam pin and both symlink
characterisation tests), and `mise run check` exits 0.

### Implementation Status

- ✓ Phase 1: Coordinated marker rename — fully implemented.

Every sub-change landed as specified:

- Shared `INSECURE_MARKER_RELATIVE` constant defined in `credentials.rs:136`
  and re-exported at the crate root (`lib.rs:28`).
- Three production builders and two contract harnesses reference the constant,
  not a literal (`jira-cli/src/context.rs`, `linear-cli/src/context.rs`,
  `work-cli/src/tracker_registry.rs`, `jira-client`/`linear-client`
  `tests/contract.rs`).
- Three flat fixtures renamed to the bare `allow-insecure-local` basename.
- Resolver docstring names the new path (`credentials.rs:380`).
- Seam pin plus two Unix-gated symlink tests added.
- Both `SKILL.md` override passages corrected and symmetric.

### Automated Verification Results

| Check | Command | Status |
| --- | --- | --- |
| Legacy literals gone | `rg insecure-local-ok` / `rg .claude/allow-insecure-local` | ✅ no matches |
| Constant value | `rg 'INSECURE_MARKER_RELATIVE.*".accelerator/allow-insecure-local"'` | ✅ matches |
| Five callers reference constant | `rg INSECURE_MARKER_RELATIVE` across builders + harnesses | ✅ all five |
| Docs name new path | `rg -c allow-insecure-local SKILL.md` = 2; docstring + constant | ✅ 2 + both |
| Warn wording gone | `rg "warns if looser than" SKILL.md` | ✅ no matches |
| Marker not gitignored | `git check-ignore .accelerator/allow-insecure-local` | ✅ exit 1 |
| Credential tests | `cargo test -p tracker-support --test credentials` | ✅ 22 passed |
| Rust + full mirror | `mise run cli:check`, `mise run check` | ✅ exit 0 |

The `mise run check` run emits one `public-api:check` rustdoc warning
(`corpus/src/frontmatter_validation/template_shape.rs:611`, unclosed `<name>`
HTML tag). It is pre-existing, untouched by this change, and does not fail the
run (`check` exits 0).

### Code Review Findings

#### Matches Plan:

- Constant, re-export, and one-line rustdoc match §1 exactly.
- All three builders and both harnesses repointed per §§2 and 5, with the
  literal/constant split the plan specified (bare basename for flat fixtures,
  full-path constant for real-root harnesses).
- The one write site
  (`the_insecure_override_needs_both_the_variable_and_a_tracked_marker`) keeps
  its bare basename and needs no `create_dir_all` — passes.
- Both symlink tests use the `#[cfg(unix)]` split and
  `std::os::unix::fs::symlink`, asserting `LocalPermsInsecure`, per §6.
- Jira `SKILL.md` sentence corrected from "warns" to refuse-plus-override,
  now symmetric with Linear.

#### Deviations from Plan:

- ⚠️ **`public-api.txt` was regenerated (+2 lines); the plan claimed the
  snapshot was unaffected.** Key Discovery at plan `:92-94` asserted "no
  `cargo-public-api` regeneration". Adding a `pub const` to the public surface
  necessarily changes the snapshot, so the implementer correctly regenerated
  it (`tests/fixtures/public-api.txt:111,352`). Benign — the plan's premise was
  inaccurate, and the correct action was taken. Without it, `public-api:check`
  would have failed.

#### Potential Issues:

- None. Override security semantics are byte-for-byte unchanged; only the
  supplied path moved. The path-agnostic resolver gates are untouched and now
  carry two additional characterisation tests closing a standing coverage gap.

### Manual Testing Required:

The plan's four manual-verification items exercise runtime override behaviour
in a scratch repo — not reachable from static validation. Recommended before
relying on the feature:

1. Override honoured:
  - [ ] `ACCELERATOR_ALLOW_INSECURE_LOCAL=1` + committed non-symlink
    `.accelerator/allow-insecure-local`, non-`0600` `config.local.md` → credential read.
2. Legacy path refused:
  - [ ] Same env var, marker only at `.claude/insecure-local-ok` → refuses with
    `E_LOCAL_PERMS_INSECURE`.
3. Symlinked / untracked marker refused:
  - [ ] Marker at new path but symlinked, or present-but-untracked → refuses.
4. Docs symmetry:
  - [ ] Jira and Linear override passages read side by side describe the same
    behaviour and name `.accelerator/allow-insecure-local`.

Note: items 1–3 are indirectly covered by the passing unit suite (override,
tracked, untracked, non-`0600`, and both symlink gates), so residual manual
risk is low.

### Recommendations:

- Merge as-is; no code changes required.
- Optional: correct the plan's Key Discovery `:92-94` to record that the
  `pub const` does change the public-API snapshot, so the "unaffected" claim
  does not mislead a future reader.
- The pre-existing `template_shape.rs:611` rustdoc warning is unrelated to this
  work but worth a separate cleanup — it is one `-D warnings` policy change away
  from breaking `public-api:check`.
