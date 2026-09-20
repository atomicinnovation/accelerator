---
type: "plan-validation"
id: "2026-09-11-0229-per-tracker-pull-scope-configuration-validation"
title: "Validation Report: Per-Tracker Pull Scope Configuration Implementation Plan"
date: "2026-09-20T18:04:49+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-09-11-0229-per-tracker-pull-scope-configuration"
tags: ["sync", "scoping", "tracker", "configuration", "pull", "jira", "linear"]
last_updated: "2026-09-20T19:17:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Per-Tracker Pull Scope Configuration

All eight phases are implemented, each as a dedicated commit, and every automated
success criterion the plan lists is backed by passing tests. The read-only CI gate
(`cli:check`, `public-api:check`) is green, the full 0229 crate test set passes
(1462 tests), and the full unit suite now passes end-to-end (3128 tests). The result
is **partial**, not pass, for two reasons: one specified deliverable — the
`config dump` override annotation — was not built, and all live-tracker manual
verification is outstanding. Three deviations from the plan text exist; two are
improvements, one is the small gap above. The plan is left `ready`, not `done`.

An unrelated pre-existing flake initially masked the full-suite result: three
`design-adapters::spawn_properties` child-spawn tests failed under full-suite load.
It was root-caused (a too-tight 10s bounded wait for a `setsid`-detached child that
is starved past the deadline only under the whole instrumented suite's fork
contention) and fixed by widening that bound to 60s; the full suite then ran green.
The flake never touched 0229 code — `design-adapters` is absent from the diff.

### Implementation Status

Each phase landed as its own commit (`tskrqnqo`..`urwoyprr`), plus a doc-comment
cleanup (`uonqntuz`). The span is 81 files, ~5.3k insertions, across exactly the
crates the plan names.

- ✅ Phase 1: Structured config block read and typed `pull` parse — fully implemented
- ✅ Phase 2: Configure-time structural validation — implemented (schema consolidated; see Deviations)
- ✅ Phase 3: `unlimited` sentinel and config-sourced ceilings — fully implemented
- ✅ Phase 4: Fail-loud on the two silent truncations — fully implemented
- ✅ Phase 5: Filters end-to-end (OR-within-a-key) — fully implemented
- ✅ Phase 6: `additional_*` multi-scope discovery — fully implemented
- ✅ Phase 7: `all_*` whole-workspace discovery — implemented (gate relocated; see Deviations)
- ✅ Phase 8: Discovered-set dedup and total ordering — fully implemented

### Automated Verification Results

- ✅ `mise run cli:check` — exit 0 (workspace-wide rustfmt + clippy `-D warnings` + drift guards)
- ✅ `mise run public-api:check` — exit 0 (all pinned snapshots match: `config`, `work`, `tracker`, `tracker-support`)
- ✅ Scoped `cargo nextest` over the 11 0229 crates — **1462 passed, 0 failed, 1 skipped**
- ✅ `mise run test:unit:cli` (full suite) — **3128 passed, 1 skipped** (after fixing an unrelated flake)

The full suite first failed on three `design-adapters::spawn_properties` child-spawn
tests (`the_child_inherits_an_owner_only_umask`,
`a_child_that_never_publishes_still_received_its_identity`,
`the_child_reads_back_the_identity_the_launcher_wrote`) with `"...never received
content"` at a 10s deadline. Investigation ruled out a code fault or sandbox block —
the tests pass in isolation (~1s) and under CPU/coverage stress, failing only under
the whole suite's fork contention, where a `setsid`-detached child's first scheduling
slice can land past 10s. The bounded wait was widened to 60s (still bounded, so a
genuine hang still fails), and the full suite then passed end-to-end. `design-adapters`
is absent from the 0229 diff, so this was never a mark against the plan's code.

The scoped run exercised the 0229 behaviour directly: 67 `pull*` tests, 10 `max_items`,
9 `max_pages`, 9 `additional_*`, 29 `filter*`, and the `unbounded` gate. The single
skipped item is the credential-gated live `tracker-contract` binary, filtered out by
`profile.default.default-filter` as designed.

### Code Review Findings

#### Matches Plan

- **Phase 1.** `Value::Mapping(Vec<(String, Value)>)` added `#[non_exhaustive]`; `project()`
  builds it recursively; the three in-crate exhaustive matches handle it
  (`as_string_sequence`→empty list, `render_value`→empty string). `PullConfig`/`PageCaps`/`parse`
  live in `cli/work/src/pull.rs`; `parse` normalises both Jira and Linear nouns to
  `additional_entities`, rejects a non-mapping block (`NotAMapping`, never silently empty),
  and collects unknown keys into a typed field. `work→config` edge added; `cli/pup.ron`
  allow-list relaxed with a revised rationale that does not restate the "zero-dependency"
  wording. Whole-block replacement falls out of `config.get(key, None)`.
- **Phase 2.** All eight rejection branches present with distinct errors, each naming the
  offending value/key, the accepted set, and the resolving config level/file via
  `Level::filename()` (the personal file when personal shadows team). Wired into both the
  config command (`PullConfigError`→`ConfigError::Invalid`→`Failure::Refusal`, fatal under
  `--fail-safe`) and the sync read path before any consumer touches the block.
- **Phase 3.** `Ceiling { Bounded(usize), Unlimited }` in `tracker` with the pinned derives;
  `TransportConfig` carries separate discovery and keyed-read caps; a single `to_ceiling`
  conversion is the sole authority and rejects a page-cap `Bounded(0)`; named
  `DEFAULT_MAX_PAGES = Bounded(50)` and `DEFAULT_MAX_ITEMS = Bounded(25)`; `--max-pulls`/
  `--max-pushes` become `Option<usize>` with no clap default; the refusal message names the
  resolved key, config level, and the `unlimited` valve. All four paging loops break on
  cursor exhaustion before the cap, so `Unlimited` pages to exhaustion safely.
- **Phase 4.** Tri-state `Completeness { Complete, CapHit, Transient }` on both `FetchOutcome`
  and `Discovery`; Jira `fetch_chunk` cap-hit is its own variant, Linear `page_all` separates
  cap-hit from deadline/wire; the fold (`Completeness::merge`) is cap-hit-dominant; a distinct
  hard-abort field on `GatheredFacts` maps to a zero-write `RunError` before planning; cap-hit
  empties `absent`; transient/out-of-scope stay non-aborting. Dedicated exit codes:
  `SEARCH_CAP_HIT` (standalone search) and `KEYED_READ_CAPPED=7` (distinct from exit-4). All
  three skills updated to branch on the cap-hit code with the `max_pages`/`unlimited` remedy;
  `sync-work-items` documents the abort and the no-`--push-only` note.
- **Phase 5.** Jira groups same-key values into one `IN` family via an explicit
  config-key→JQL-field mapping (`label`→`labels`, `state`→`status`), a safe-identifier
  assertion at the `format!` sink (`BadJql`), and `quote()` backslash escaping per JQL
  grammar. Linear lowers multi-value keys to `in`, single values keep `eq`, keys AND'd in one
  `IssueFilter`. Filters flattened into `SearchScope.filters`, replacing `Vec::new()`. Goldens
  and MockServer contract tests (incl. adversarial break-out values) added.
- **Phase 6.** `SearchScope` moved to the exclusive `EntityScope { Keyed { base, additional },
  WholeWorkspace }` sum type; a dyn-compatible `enumerate_visible_entities` port method; a
  single shared resolver (`sync/scope.rs`) enumerates before search, confirms membership, and
  routes a genuine miss to `RunError::DiscoveryUnconfigured` (exit-74, no new `TrackerError`
  variant) versus a transient `Retryable`. Linear enumeration paginates to exhaustion;
  `CatalogueTeam` grew to a multi-entry map, growing lazily inside one `with_lock` closure and
  preserving old fields; the Linear keyed read is broadened per catalogued team with widened
  `in_scope`.
- **Phase 7.** `all_entities`→`WholeWorkspace` (exclusivity structural); adapters emit an
  enumerated `IN` list, never an empty/unbounded query; Linear relaxes its base-team guards
  under the variant while still refusing an empty `Keyed`. The unbounded-write gate fires on
  `unlimited` + any broadened scope (`all_*` or non-empty `additional_*`), the binary refuses
  fail-safe with `REFUSED_UNBOUNDED=8`, and the skill drives the `AskUserQuestion` +
  `--allow-unbounded` re-run, mirroring the exit-5 gate.
- **Phase 8.** Dedup by `canonical_external_key` precedes the local subtraction; the
  `discovered_order` comparator sorts by `(prefix, sequence, raw id)` with a final-`-` split,
  case/whitespace fold matching the dedup fold, and saturation to `usize::MAX`; `ExternalId`
  gains no `Ord`; `max_items` counts the post-dedup, post-subtraction set.

#### Deviations from Plan

- **Filter schema consolidated into `work`, not per-client instances (Phase 2).** The plan
  specified `const FILTER_SCHEMA` instances in new `cli/jira-client/src/pull.rs` and
  `cli/linear-client/src/pull.rs`, with `validate(&PullConfig, &FilterSchema)`. The
  implementation keeps a single shared `FILTER_SCHEMA` in `cli/work/src/pull.rs` and
  `validate(&PullConfig, Tracker)` selects the schema internally. The client `pull.rs` files
  do not exist. Justified in-code by a YAGNI note (the accepted set does not diverge per
  tracker today); all rejection behaviour, including the wrong-tracker-noun hint, is present.
  Improvement over the plan for today's needs; revisit if the accepted sets diverge.
- **`config dump` override shown by per-row source only (Phase 1.4).** The plan asked for a
  distinct annotation naming the dropped team-only fields when a personal block shadows a
  team block. The implementation surfaces the override through each row's `local (...)` source
  rather than a dedicated callout (an in-code comment acknowledges this). This is exactly the
  one Phase 1 manual box the plan itself left unchecked (plan line 350).
- **Unbounded-write gate located in `work-cli`, not `work-adapters` (Phase 7.3).** The plan
  placed the gate in `cli/work-adapters/src/sync/`; the firing predicate and refusal message
  live in `cli/work-cli/src/sync.rs` (invoked pre-flight before `work_adapters::sync::run`),
  with only the `is_broadened` predicate in `work-adapters`. Behaviour is identical
  (fail-safe refusal, dedicated code, no TTY prompt); the placement is arguably cleaner.

#### Potential Issues

- **A pre-existing spawn-test flake was fixed en route (resolved).** Three
  `design-adapters::spawn_properties` tests intermittently missed a 10s bounded wait under
  full-suite fork contention; the bound was widened to 60s and the full suite now passes
  (3128 tests). Unrelated to 0229, and now green rather than an open risk.
- **Live-tracker behaviour is unverified.** Every filter-lowering, entity-enumeration, and
  cap-hit path is pinned by goldens and offline contract tests only. The plan's own
  Phase 5/6 notes call for a live schema check that Linear's `IssueFilter` accepts `in` on
  `labels.name`/`state.id`/`team.id`, and that a hostile Jira filter value stays contained at
  the remote — neither can be confirmed without credentials.

### Manual Testing Required

The plan's manual-verification boxes are largely unchecked because they need live Jira/
Linear credentials and a real corpus. Recommended before release:

1. Config surface:
   - [ ] `accelerator config dump` shows a configured `pull` block and an unset-but-available placeholder.
   - [ ] A personal `<tracker>.pull` block visibly replaces the team block in `dump` (note: shown via per-row source, not a dedicated annotation — see Deviations).
   - [ ] An unsupported filter key / reserved `all` / bad ceiling / `all_*`-with-`additional_*` each rejected at `configure`, non-zero, with an actionable message.
2. Ceilings and truncation:
   - [ ] `max_items` below the discovery count refuses a `--preview` with zero writes; `unlimited` lifts it.
   - [ ] `accelerator jira search` / `accelerator linear search` past the cap exits non-zero and names `max_pages`.
   - [ ] A sync whose keyed read truncates aborts rather than marking items awaiting-human.
3. Scope broadening:
   - [ ] A pull with `filters` returns only issues matching the AND/OR semantics per tracker.
   - [ ] `additional_projects` / `additional_teams` discovers from base plus each additional entity; a misspelled entity aborts with a clear message.
   - [ ] `all_projects` / `all_teams` discovers across the whole visible workspace, halting on `max_items`/`max_pages` rather than flooding; the `--allow-unbounded` gate engages under `unlimited` + broadened scope.
   - [ ] A multi-scope pull spanning several prefixes reconciles in the documented order with no duplicate imports.

### Recommendations

- Run the bare `mise run` default task (frontend + docs lanes) on CI before merge; the
  `cli:check`, `public-api:check`, and full `test:unit:cli` lanes are already confirmed green.
- Execute the manual live-tracker checklist above against a scratch Jira and Linear
  workspace, prioritising the adversarial filter-value and the whole-workspace flood
  paths.
- Accept or record the three deviations. The schema consolidation and gate relocation are
  improvements; the `dump` override annotation is a small DX gap already tracked as an
  Open Item (plan "DX polish") and can be deferred.
