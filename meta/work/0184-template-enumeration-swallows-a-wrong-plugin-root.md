---
type: "work-item"
id: "0184"
title: "Template resolution succeeds silently on a plugin root that is not an installation"
date: "2026-07-29T00:00:00+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "ready"
kind: "bug"
priority: "low"
parent: "work-item:0136"
relates_to: ["work-item:0182"]
tags: ["bug", "cli", "config", "templates", "plugin-root"]
last_updated: "2026-09-09T23:33:52+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "AC12 ticked: after the docs-site dependency rebase resolved docs:audit:check, a full mise run surfaced two further deterministic failures the implementation had to fix — the config public-api fixture (the new PluginRootNotAnInstallation variant) and the shutdown integration tests (their templates-less plugin-root fixture, which compose now correctly refuses). Both fixed; a clean mise run then exited 0 end-to-end, so every acceptance criterion (AC1–AC13) is now satisfied."
schema_version: 1
external_id: "PP-714"
---

# 0184: Template resolution succeeds silently on a plugin root that is not an installation

**Kind**: Bug
**Status**: Ready
**Priority**: Low
**Author**: Toby Clemson

## Summary

Three plugin-root template accessors in `FileConfigStore`
([`cli/config-adapters/src/store.rs`](../../cli/config-adapters/src/store.rs))
now refuse when the plugin root is *absent*, but still succeed silently when
the root is *present but carries no `templates/` directory* — present, yet (by
the Assumption below) not an Accelerator installation. Such a root is `Some(path)`,
so it passes `require_plugin_root` and falls through each accessor's own
filesystem check. "Wrong root" throughout this item means precisely the
detectable structural cases: a present root whose `templates/` is missing or is
not a directory, or a root that is itself a file. A mis-pointed
root that happens to retain an unrelated `templates/` directory is not
distinguishable from a valid installation by this signal and is out of scope:

- `template_names` swallows every `read_dir` failure into an empty list:

```rust
let Ok(entries) = fs::read_dir(plugin.join("templates")) else {
    return Ok(Vec::new());
};
```

- `resolve_template`'s plugin-default tier returns `Ok(None)` when `<name>.md`
  is not a file.
- `plugin_template_path` tests nothing; its callers `plugin_default` and
  `eject` do the `is_file()` check and fall through to `Ok(None)` /
  `EjectOutcome::NoDefault`.

The observable for `template_names` is a header-only `config templates list`
table at exit 0 — the same silent-wrong-answer mode 0182 closed for the
absent-root case, surviving one step further out. The other two accessors
degrade a wrong-root diagnosis into a template-name "not found".

`template_names` is also consumed by the visualiser server's compose path
([`cli/visualiser/server/src/compose.rs`](../../cli/visualiser/server/src/compose.rs)),
which forwards `ConfigError` via `?` into `ComposeError`. That consumer is in
scope: once the accessor refuses on a wrong root, the server must surface the
refusal rather than swallow it into an empty template set.

## Context

0182 (now `done`) Phase 5 introduced `ConfigError::PluginRootUnavailable` and
the `require_plugin_root` gate, which these template accessors already call so
they refuse on an *absent* root, and deliberately left the *wrong-root* arms
alone so the phase stayed scoped to the root's *presence*. The surviving `template_names` behaviour is pinned as a
characterisation test —
`a_root_without_a_templates_directory_still_renders_an_empty_table` in
`cli/launcher/tests/config_read.rs` — so it is visible rather than implicit, and
that test is what this item would change.

A wrong root became newly plausible in the same change: the bootstrap derives
the root from its own location, and a directly-invoked launcher passes no
`plugin.json` gate that would catch a root pointing somewhere unexpected.

## Requirements

- **R1 — `template_names`**: distinguish the failure kinds instead of
  collapsing them. A *structural* wrong root — `templates/` missing
  (`ErrorKind::NotFound`), a `templates` entry that is a file, or a root that is
  itself a file (`ErrorKind::NotADirectory`) — is not an installation and becomes
  `ConfigError::PluginRootNotAnInstallation`, a refusal naming the offending root
  and `ACCELERATOR_PLUGIN_ROOT`. Only a genuinely *unreadable* fault (permission
  denied, filesystem loop) becomes `ConfigError::Io` naming the path — reserving
  the degradable class for the one case where the root's validity is
  undeterminable. The `ConfigError::Io` half constructs via the `io_error` helper
  as `custom_lenses` and `skill_names` do; the refusal half is new and rests on
  the Assumption below — those siblings enumerate the project config directory and
  map a missing directory to an empty list, so this is a deliberate divergence,
  not parity. `template_names` is enumerated by two commands, `config templates
  list` and `config templates eject --all` (via `eject_all`), so this one fix
  covers both.
- **R2 — `resolve_template` plugin-default tier and `plugin_template_path`**:
  distinguish "the root is not an installation (structurally-invalid
  `templates/`)" (→ `ConfigError::PluginRootNotAnInstallation`, whose diagnostic
  names the offending root and `ACCELERATOR_PLUGIN_ROOT`) from "`<name>.md` is
  genuinely absent inside a valid installation" (→ `Ok(None)`, a genuine
  template-not-found). Today both cases
  collapse to `Ok(None)` / `EjectOutcome::NoDefault`, which names the template
  rather than the root. Apply the distinction at each plugin-default site —
  `resolve_template`'s plugin-default tier (reached by `config template <name>`),
  and `plugin_template_path`'s callers `plugin_default` and `eject`. `eject`
  refuses on a wrong root (`PluginRootNotAnInstallation`) rather than treating it
  as "nothing to eject". Note the `resolve_template` **plugin-default tier** and the
  method **`plugin_default`** are two distinct sites despite the near-identical
  names: the tier is reached by `config template <name>`, whereas `plugin_default`
  is reached only by `config templates diff <name>` and `config templates reset
  <name>` — those two commands are the observable surface for its half of the fix
  (see Technical Notes for the full command-to-site mapping).
- **R3 — Visualiser server compose path**: the server's `compose.rs`, which
  forwards `ConfigError` via `?` into `ComposeError`, must propagate the wrong-root
  refusal as a `ComposeError` whose diagnostic names `ACCELERATOR_PLUGIN_ROOT`
  rather than composing against an empty template set. The `?` propagation likely
  needs no production change; the requirement is to confirm the refusal reaches
  the server boundary intact and to cover it with a test.
- **R4 — Keep the root-independent families and project-local overrides working**:
  the root-independent config families (`agents`, `paths`, and the like) never
  read the plugin root, so a wrong root leaves them unaffected — no change is
  required of them, and that is the behaviour to preserve. A wrong root must
  likewise not break `config template <name>` when a user override resolves: the
  override tiers are checked before the plugin-default tier, so a resolving
  override never reaches the wrong root — the property
  `a_user_override_still_resolves_with_no_plugin_root` pins this for the
  absent-root case.

## Acceptance Criteria

Every criterion below operationalises "wrong root" as a root that exists but is
not an installation — its `templates/` is missing or not a directory, or the root
is itself a file.

- [x] **AC1** — `config templates list` exits non-zero with a diagnostic naming
      `ACCELERATOR_PLUGIN_ROOT`, replacing the header-only table at exit 0.
      (Exercises `template_names`.)
- [x] **AC2** — `config template <name>`, with no resolving override, exits
      non-zero naming `ACCELERATOR_PLUGIN_ROOT`, replacing today's `Ok(None)`
      rendered as a template-name "not found". (Exercises `resolve_template`'s
      plugin-default tier.)
- [x] **AC3** — `config templates eject <name>` exits non-zero with a diagnostic
      naming `ACCELERATOR_PLUGIN_ROOT` (surfacing
      `ConfigError::PluginRootNotAnInstallation`), not `EjectOutcome::NoDefault` —
      the wrong root is diagnosed rather than reported as "nothing to eject".
      (Exercises `eject` → `plugin_template_path`.)
- [x] **AC4** — `config templates eject --all` exits non-zero with a diagnostic
      naming `ACCELERATOR_PLUGIN_ROOT`, replacing today's "exit 0 having ejected
      nothing". (Exercises `eject_all` → `template_names`, the second consumer.)
- [x] **AC5** — `config templates diff <name>` and `config templates reset
      <name>` each exit non-zero with a diagnostic naming
      `ACCELERATOR_PLUGIN_ROOT`, replacing today's `Ok(None)` rendered as an
      unknown-template error. (These are the observable surface of
      `plugin_default` → `plugin_template_path`.)
- [x] **AC6** — invoking the visualiser server's compose path (`compose.rs`)
      against a store whose plugin root is a wrong root returns
      `Err(ComposeError)` whose message contains `ACCELERATOR_PLUGIN_ROOT` and
      produces no empty template set — asserted by a test at the compose-path
      boundary.
- [x] **AC7** — `config template <name>` against an installation whose
      `templates/` directory is present but lacks `<name>.md`, with no resolving
      override, exits non-zero with a not-found message naming `<name>` and
      **not** mentioning `ACCELERATOR_PLUGIN_ROOT` — so the genuine
      template-not-found is distinguished from the plugin-root refusal by message
      content, not merely by the two messages differing.
- [x] **AC8** — a genuine *unreadable* fault yields a `ConfigError::Io` naming the
      path, not the structural refusal, and does **not** name
      `ACCELERATOR_PLUGIN_ROOT` — so a fault whose validity is undeterminable stays
      degradable. Trigger it with an environment-independent, root-guard-free
      fault: a self-referential `templates` symlink, whose `fs::metadata` fails
      `FilesystemLoop` (neither `NotFound` nor `NotADirectory`). The
      `templates`-is-a-file shape is **not** this case — it is a structural refusal
      under AC13.
- [x] **AC9** — the characterisation test
      `a_root_without_a_templates_directory_still_renders_an_empty_table` is
      replaced at the same site by a test asserting the empty-`templates/` root
      now yields a non-zero exit whose diagnostic names `ACCELERATOR_PLUGIN_ROOT`
      (the inverse of AC1), rather than deleted.
- [x] **AC10** — given a wrong root and a user override configured for `<name>`,
      `config template <name>` exits 0 and renders the override's content, not the
      `ACCELERATOR_PLUGIN_ROOT` refusal. This is the wrong-root case, distinct
      from the absent-root property
      `a_user_override_still_resolves_with_no_plugin_root`, and needs its own
      coverage.
- [x] **AC11** — a root-independent family command (e.g. `config paths`) against
      a wrong root exits 0 **and** renders its normal non-empty output (not an
      empty or notice-only degradation), exercising R4: the families that never
      read the plugin root are unaffected by the refusal. Exit 0 alone is
      insufficient because the `config` family carries `--fail-safe`.
- [x] **AC12** — `mise run` (bare default task) exits 0 end-to-end.
- [x] **AC13** — the two remaining structural shapes each fail closed: a root
      whose `templates` entry is a file, and a root that is itself a file, make
      `config templates list` exit non-zero with a diagnostic naming
      `ACCELERATOR_PLUGIN_ROOT` — a structural wrong root is refused, not reported
      as an `Io` fault. Both triggers are environment-independent (no permission
      fault, no root guard). This distinguishes the structural shapes (refuse) from
      a genuinely unreadable fault (AC8, `Io`).

## Decisions

No open questions remain. The decisions taken during review:

- **Detection lives in a single shared "is this root an installation?" helper**
  that validates `templates/` presence and is called by all three accessors,
  rather than an inline check duplicated per site. This shared helper is why the
  accessors stay bundled in one item rather than split.
- **`eject` against a wrong root refuses** with
  `ConfigError::PluginRootNotAnInstallation` rather than reporting
  `EjectOutcome::NoDefault`; pinned by its own acceptance criterion above.
- **The visualiser server compose path is in scope** and refuses identically,
  rather than being deferred to a follow-up — the change would otherwise switch
  the server from an empty template set to a hard error unremarked.
- **The wrong-root refusal uses a dedicated `PluginRootNotAnInstallation { path }`
  variant**, not a reused `PluginRootUnavailable`. The latter's message says the
  root is *unknown* and to *set* the variable — misleading for a developer who set
  it to a real path that simply is not an installation. The dedicated variant
  names the offending root and the missing-`templates/` cause. (Plan-review
  decision, 2026-09-09.)
- **Every structural wrong-root shape fails closed.** Missing `templates/`, a
  `templates` that is a file, and a root that is itself a file all refuse with
  `PluginRootNotAnInstallation`; `Io` is reserved for a genuinely *unreadable*
  fault (permission, filesystem loop), the one case where the root's validity is
  undeterminable. This keeps every definitely-wrong root fail-closed even under
  `--fail-safe`. (Plan-review decision, 2026-09-09; refines AC8, adds AC13.)

## Dependencies

- Builds on: 0182 (`done`) — introduced `ConfigError::PluginRootUnavailable` and
  the `require_plugin_root` gate that all three template accessors already call to
  refuse on an *absent* root. This item extends the same refusal to the
  *wrong-root* case (present, no `templates/`): 0182 handled the absent case,
  0184 handles the wrong case, across the same accessors — the "two named
  accessors" of an earlier draft was an error, the count is three.
- Affected consumers of the changed accessors — beyond the primary `config
  templates list` / `config template <name>` surfaces, three consumers change
  behaviour on a wrong root and are pinned by criteria above:
    - `config templates eject --all` (`eject_all`, `inbound/cli.rs:542`)
      enumerates via `template_names`, so it flips from "exit 0 having ejected
      nothing" to a refusal (AC4).
    - `config templates diff <name>` / `reset <name>` reach `plugin_default`, so
      they flip from an unknown-template error to a refusal (AC5).
    - the visualiser server's `cli/visualiser/server/src/compose.rs` forwards
      `template_names`' `ConfigError` via `?` into `ComposeError` (named in the
      0182 plan, Phase 5 §2), so it flips from an empty template set to a hard
      error; brought into scope (R3, AC6) rather than left unremarked. The
      visualiser frontend consuming that `ComposeError` is the ultimate
      downstream — its surface-level handling of a compose error is out of scope
      here.
- Distribution — inherits 0182's launcher/server lockstep. The change alters a
  shared library (`config-adapters`) whose behaviour ships in two binaries (the
  launcher and the visualiser server); both must ship at the same plugin version,
  which 0182's version-keyed cache already guarantees. Noted so the two-binary
  coupling is on the record.
- Precondition — the `NotFound → PluginRootNotAnInstallation` mapping assumes every
  Accelerator installation ships a `templates/` directory (see Assumptions).
  Confirm this against the release artifact's file list before implementation; if
  an installation can ship without `templates/`, the absence signal is unreliable
  and the mapping must be reconsidered.

## Assumptions

- Every Accelerator installation ships a `templates/` directory, so its absence
  is a reliable signal that the root is not an installation. Confirming this
  against the release artifact's file list is tracked as a pre-implementation
  precondition under Dependencies.

## Technical Notes

- Sites: `template_names` (`store.rs:412`), `resolve_template`'s plugin-default
  tier (`store.rs:400`), and `plugin_template_path` (`store.rs:466`) with its
  callers `plugin_default` (`store.rs:436`) and `eject` (`store.rs:483`).
- Command-to-site mapping (the observable surface for each store site):

  | Command | Store site | Wrong-root outcome today |
  |---------|------------|--------------------------|
  | `config templates list` | `template_names` | empty table, exit 0 |
  | `config templates eject --all` | `eject_all` → `template_names` | ejects nothing, exit 0 |
  | `config template <name>` | `resolve_template` plugin-default tier | `Ok(None)` → not-found |
  | `config templates eject <name>` | `eject` → `plugin_template_path` | `EjectOutcome::NoDefault` |
  | `config templates diff <name>` / `reset <name>` | `plugin_default` → `plugin_template_path` | `Ok(None)` → unknown-template |
  | visualiser compose | `resolve_templates` → `template_names` | empty template set |

  `config template <name>` does **not** reach `plugin_default`; `diff`/`reset`
  are its only CLI surface (`inbound/cli.rs:579,602`).
- `require_plugin_root` (`store.rs:73`) is the only gate that raises
  `PluginRootUnavailable`, and it fires solely on an *absent* root; a
  wrong-but-present root is `Some(path)` and passes it.
- `ConfigError::Io { path, detail }` is constructed via the `io_error` helper
  (`store.rs:729`); `custom_lenses` (`store.rs:294`) and `skill_names`
  (`store.rs:320`) show the `match … ErrorKind::NotFound` split to follow for
  the I/O half.
- Server consumer: `cli/visualiser/server/src/compose.rs` calls `template_names`
  and forwards its `ConfigError` via `?` into `ComposeError`. Verify the
  `PluginRootNotAnInstallation` diagnostic survives that conversion to the server
  boundary and is not flattened into a generic compose failure.

## Drafting Notes

- Corrected two behavioural claims in the prior draft against `store.rs`:
  `resolve_template`'s plugin-default tier returns `Ok(None)` for a wrong root
  (it does not name the template), and `plugin_template_path` tests nothing —
  its callers `plugin_default`/`eject` perform the `is_file()` check.
- Scope extended to all three plugin-root accessors, beyond the original
  `template_names`-only bug — the "Option B" choice, where **Option A** was to fix
  `template_names` alone and **Option B** to fix all three accessors together.
  This enlarges the item and motivated the title change; a reviewer may still
  prefer to split the `resolve_template`/`plugin_template_path` work into its own
  item.
- Reframed the "matching the siblings" justification: parity with
  `custom_lenses`/`skill_names` holds only for the `ConfigError::Io` half; the
  `NotFound → PluginRootUnavailable` mapping is new and rests on the Assumption.
- Reconciled the dependency on 0182: it is now `done`, so the body no longer
  states it blocks this item, matching the frontmatter `relates_to`.
- Review 1 (REVISE) revisions: resolved the `eject` semantics to *refuse*
  (`PluginRootUnavailable`) and pinned it with a dedicated acceptance criterion;
  narrowed the Summary's "wrong root" to the detectable no-`templates/`-directory
  case so intent matches the criteria (a wrong root that retains a `templates/`
  directory is explicitly out of scope); restated AC2 to match AC1's wording and
  pinned AC3 to concrete message content; removed the "three sites" count
  collision in Requirement 2; and promoted the `templates/`-presence check to a
  tracked precondition under Dependencies. Scope kept bundled per the
  single-shared-helper rationale.
- The filename slug (`template-enumeration-...`) reflects the item's original
  `template_names`-only framing; the kept-bundled scope and the title span
  `resolve_template`/`plugin_template_path` too. The slug is retained to preserve
  the stable path and inbound references from 0182.
- Review 1 pass 2 (REVISE) revisions: brought the visualiser server compose path
  (`compose.rs`) into scope to refuse identically on a wrong root, adding a
  requirement, an acceptance criterion, a Dependencies consumer entry and a
  Technical Note — the item now spans the `config-adapters` and `visualiser`
  server crates rather than `store.rs` alone. Restated the user-override criterion
  as a full input/action/expected triple; re-pinned the eject criterion to a
  CLI-observable diagnostic naming `ACCELERATOR_PLUGIN_ROOT` and standardised that
  wording across the plugin-root criteria; tied the characterisation-test
  replacement to the concrete AC1 assertion; noted the non-root constraint on the
  `0o000` I/O trigger; and retitled "Open Questions" (which held only resolved
  items) to "Decisions".
- Review 1 pass 3 (REVISE) revisions, after reading the call graph in `store.rs`
  and `inbound/cli.rs`: reconciled the accessor count (0182's `require_plugin_root`
  gate already makes the three accessors refuse on an *absent* root; this item
  extends the refusal to the *wrong-root* case — the earlier "two named accessors"
  was wrong). Labelled the Requirements R1–R4 and the Acceptance Criteria AC1–AC12
  so every ordinal cross-reference resolves. Added AC4 (`eject --all`, a second
  `template_names` consumer via `eject_all`), AC5 (`diff`/`reset`, the CLI surface
  of `plugin_default` — `config template <name>` does not reach `plugin_default`),
  and AC11 (a root-independent family still succeeds), plus a command-to-site
  mapping table in Technical Notes and the corresponding consumer and
  distribution-lockstep entries in Dependencies. Pinned the `eject` command forms,
  standardised the absent-root term, and defined Option A/B.
- Plan-review (2026-09-09 review-1, pass 2) revisions, folded back from the plan:
  (1) the wrong-root refusal now uses a dedicated
  `ConfigError::PluginRootNotAnInstallation { path }` variant naming the offending
  root — the reused `PluginRootUnavailable` message ("unknown: set
  ACCELERATOR_PLUGIN_ROOT") misdescribes a set-but-wrong root; R1/R2/AC3/Decisions
  updated. (2) Every *structural* wrong-root shape fails closed — `templates/`
  missing, a `templates` that is a file, and a root that is itself a file all
  refuse — with `Io` reserved for a genuinely unreadable fault (permission,
  filesystem loop); AC8's trigger became a filesystem-loop symlink (dropping the
  earlier `0o000` root-guard concern, since the launcher test crate has no
  root-detection facility), AC13 was added for the two extra structural shapes,
  and the wrong-root definition was broadened in the Summary and the AC preamble.

- `meta/work/0182-cli-derives-plugin-root-from-own-location.md`
- `meta/plans/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename.md`
  — Phase 5 §3, "What this does and does not buy"
