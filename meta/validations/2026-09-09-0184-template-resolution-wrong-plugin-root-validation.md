---
type: "plan-validation"
id: "2026-09-09-0184-template-resolution-wrong-plugin-root-validation"
title: "Validation Report: Template Resolution Wrong-Root Refusal"
date: "2026-09-10T00:28:01+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-09-0184-template-resolution-wrong-plugin-root"
tags: ["cli", "config", "templates", "plugin-root", "visualiser"]
last_updated: "2026-09-10T00:28:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Template Resolution Wrong-Root Refusal

The plan was implemented faithfully across all three phases; every automated
criterion passes and the production code matches the plan's specified shape
line-for-line. Result: **pass**. Two beyond-plan fixture fixes were required to
green the full CI mirror — both anticipated by the work item, neither a
deviation from intent.

### Implementation Status

✓ Phase 1: `template_names` refuses on a wrong root (R1) — fully implemented
✓ Phase 2: Plugin-default resolution refuses on a wrong root (R2) — fully implemented
✓ Phase 3: Compose path surfaces the refusal (R3) — fully implemented

Each phase landed as its own commit — `Refuse template enumeration on a wrong
plugin root`, `Refuse plugin-default resolution on a wrong plugin root`, `Pin the
compose-path wrong-root refusal with a contract test` — matching the plan's
"independently mergeable, tree stays green" sequencing.

### Automated Verification Results

| Check | Command | Status |
|-------|---------|--------|
| Error-crate tests | `cargo test -p config` | 🟢 exit 0 |
| Store tests | `cargo test -p config-adapters` | 🟢 exit 0 |
| Launcher integration | `cargo test -p accelerator --test config_read` | 🟢 152 passed |
| Compose contract | `cargo test -p accelerator-visualiser --test compose_contract` | 🟢 6 passed |
| Format + lint | `mise run cli:check` | 🟢 exit 0 |

✅ The 10 new store tests and 2 new error-crate tests were confirmed by name,
not only by an aggregate exit code:

- `template_names_refuses_a_root_without_a_templates_dir`, `…_when_templates_is_a_file`,
  `…_when_the_root_is_a_file`, `…_reports_io_on_an_unreadable_templates`.
- `resolve_template_refuses_a_wrong_root_with_no_override`,
  `resolve_template_resolves_a_user_override_against_a_wrong_root` (the hoisting crux),
  `resolve_template_reports_not_found_for_a_missing_default`.
- `plugin_default_reports_not_found_for_a_missing_default`,
  `plugin_default_refuses_a_wrong_root`, `eject_refuses_a_wrong_root`.
- `plugin_root_not_an_installation_names_the_path_and_variable`,
  `a_missing_plugin_root_is_a_refusal_and_a_read_failure_is_not`.

✅ The compose boundary test `a_wrong_plugin_root_refuses_to_compose` passes,
confirming R3 needed no production change — the `?` at `compose.rs:157` already
propagates the refusal.

⚠️ AC12 (`mise run`, the full local CI mirror) I did **not** re-run — it compiles
Rust several times and needs a Chromium install for the docs lane. The work
item records it as satisfied: commit `ylsnwkwn` cleared the docs-site npm
advisories that blocked `docs:audit:check`, after which a clean `mise run`
exited 0, forcing two further deterministic fixes (see Deviations). This is the
one criterion resting on recorded evidence rather than a run in this session.

### Code Review Findings

#### Matches Plan

- **Error variant** — `PluginRootNotAnInstallation { path }` (`cli/config/src/error.rs:67`),
  its `Display` arm naming the path, the `no templates/ directory` cause, and
  `ACCELERATOR_PLUGIN_ROOT` (`:142`), and its `is_refusal() == true` classification
  alongside `PluginRootUnavailable` (`:83`) — all verbatim to the plan. `is_refusal`
  stays exhaustive (no wildcard), so the variant could not compile unclassified.
- **Shared helper** — `require_templates_dir` (`cli/config-adapters/src/store.rs:469`)
  is byte-faithful: one `fs::metadata` probe, `Ok(is_dir)` → path, `Ok(!is_dir)` →
  refusal, `NotFound | NotADirectory` → refusal, any other error → degradable `io_error`.
- **Single choke point** — `template_names` (`:412`), `plugin_template_path` (`:490`),
  `plugin_default` (via `plugin_template_path`, `:434`), `eject` (`:504`), and
  `resolve_template`'s plugin-default tier (`:400`) all route through the helper.
- **Override precedence preserved** — the config-path and user-override tiers
  (`:374-399`) sit ahead of the plugin-default check, so a resolving override still
  renders at exit 0 under a wrong root.
- **Compose propagation** — `store.template_names()?` at `compose.rs:157` unchanged;
  the refusal reaches `ComposeError::Config` intact.

#### Deviations from Plan

- **Public-api fixture regen** (`cli/config/tests/fixtures/public-api.txt`, commit
  `mlwvoozy`) — the new public variant changed the config crate's surface;
  `cargo-public-api` demanded the fixture update. Anticipated by AC12, not in the
  plan's file list. Correct.
- **Shutdown-test fixture** (`cli/visualiser/server/tests/shutdown.rs`, commit
  `yswxnryq`) — seeds `templates/` in the test's plugin-root fixture, because compose
  now correctly refuses a templates-less root. A necessary consequence of R3, not a
  behavioural change. Correct.
- **Comment stripping** (commit `rlxrmrzq`) — removed AC/work-item/phase references
  from code comments per the house "comments are a last resort" rule, net −20 lines
  across `store.rs`, `config_read.rs`, `compose_contract.rs`. Improvement.

#### Potential Issues

- **Residual layout duplication** — `compose.rs:155` still reconstructs
  `plugin_root.join("templates")` to build each tier's absolute default path,
  duplicating the layout fact the helper owns. The plan calls this a conscious
  out-of-scope choice; the `store-duplication:check` lint passes, so it is within
  tolerance. No action needed, flagged for the record.
- **AC8 semantics moved under the plan** — the work item's AC8 originally treated a
  `templates`-is-a-file root as the representative `Io` fault; the plan reclassified
  that shape as a structural refusal and reserved `Io` for genuinely-unreadable
  faults (permission, filesystem loop). The work item was updated (AC8 revised, AC13
  added) and marks AC1–AC13 satisfied. No stale criterion remains.

### Manual Testing Required

The store and CLI tests already exercise these paths; manual runs are confirmatory
only, against a real installed plugin rather than a fixture.

1. Wrong-root refusal:
  - [ ] `ACCELERATOR_PLUGIN_ROOT=$(mktemp -d) accelerator config templates list` —
        exits non-zero, names the root and `ACCELERATOR_PLUGIN_ROOT`, states "not an
        Accelerator installation".
  - [ ] Repeat for `template <name>`, `templates eject <name>`, `eject --all`,
        `diff <name>`, `reset <name>` — each refuses.
2. Preserved behaviour:
  - [ ] Against the real plugin root, `config template not-a-real-template` reports a
        template-not-found naming the template, with no mention of the plugin root.
  - [ ] `ACCELERATOR_PLUGIN_ROOT=$(mktemp -d) accelerator config paths --fail-safe` —
        exits 0 with normal non-empty output.

### Recommendations

- **No blockers to merge.** The implementation is complete, tested, and lint-clean.
- **Close the plan lifecycle** — plan status advanced to `done` by this validation.
- **Track the AC13 follow-up** noted in the plan (the two extra structural shapes and
  the AC8 revision) — already reflected in the work item; no separate action.
