---
type: "plan-review"
id: "2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids-review-1"
title: "Plan Review: Linear Pull Filters via Catalogue-Resolved Ids Implementation Plan"
date: "2026-09-23T02:00:21+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-22-0292-linear-pull-filters-catalogue-resolved-ids"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "usability", "performance"]
review_number: 1
review_pass: 6
tags: ["linear", "pull-filters", "catalogue"]
last_updated: "2026-09-23T16:53:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Linear Pull Filters via Catalogue-Resolved Ids Implementation Plan

**Verdict:** REVISE

The wire shapes, the resolve-and-refuse template, the pure tier-walking user
resolver and the no-silent-drop phase ordering are sound. Where the plan falls
short is at its boundaries. On the sync pull path, the new refusals collapse
into a "transient, retry" discovery failure. The grow-path refresh has no route
from `work-cli` to `discover_workspace`. The migration remedy wipes the grown
`teams` array. Workspace-wide label resolution turns today's working team-scoped
label names into permanent refusals. The phases are also not written test-first,
and the plan's headline "refuse, fetch nothing" property is only checked
manually.

### Cross-Cutting Themes

- **Pull-path refusals surface as transient failures** (flagged by:
  architecture, code-quality, correctness, compatibility) — `fetch_page`
  stringifies the `compose` error, `page_all` marks it `Completeness::Transient`,
  and sync reports `DiscoveryIncomplete` ("cut short… retry"). The typed
  variants, exit codes and `init-linear` remedy only reach the standalone
  `search` subcommand. `resolve_scope` is the port's existing pre-flight seam
  for exactly this kind of refusal.
- **Grow path cannot reach `discover_workspace`** (flagged by: architecture,
  compatibility, correctness, test-coverage) — `grow_linear_catalogue` only
  holds `&dyn RemoteTracker`. Extending the port churns the snapshotted
  `tracker` API, which contradicts "no public-API churn". The failure semantics
  (does the team append still commit?) are also unspecified.
- **Migration via `init-linear` clobbers `teams`, and "or a team-adding pull"
  is unreachable** (flagged by: correctness, compatibility, usability) —
  `write_catalogue` overwrites the grown multi-team array. A filtered pull
  refuses before the grow ever runs.
- **Team-scoped duplicate label names become falsely ambiguous** (flagged by:
  correctness, compatibility, usability) — workspace `issueLabels` returns every
  team's `Bug`. Today's `labels.name.eq` matched them all, and no config syntax
  can disambiguate.
- **Archived/deactivated entities excluded by default** (flagged by:
  correctness, compatibility, usability) — `includeArchived` /
  `includeDisabled` default to false, so historical values refuse, with a remedy
  that cannot help.
- **Per-family duplication and the `SEARCH_BAD_STATE` overload** (flagged by:
  code-quality, architecture, usability, compatibility) — two identical
  traits, two parallel outcome enums, six near-identical error variants, a
  "bad state" code meaning any unknown value, and an exit-code fixture that is
  the wrong place to pin a new code.
- **Silent `_ => {}` intake catch-all retained** (flagged by: code-quality,
  architecture) — the root cause of the bug stays in place for the next key
  (0293).
- **Catalogue shape knowledge scattered; five separate loads** (flagged by:
  code-quality, architecture, performance) — section keys appear as string
  literals in four places, and each resolver re-reads and re-parses
  `catalogue.json`.
- **Unlisted `compose`/`LinearClient::new` consumers** (flagged by: correctness,
  compatibility, test-coverage, usability) — `run_search` at `main.rs:113`,
  `build_with_override` in `context.rs:213`, cross-crate test supports, and
  the `search-linear-issues` skill documentation.

### Tradeoff Analysis

- **Refuse on ambiguity vs preserve today's semantics (labels)**: correctness
  and safety favour refusing. Compatibility and usability note that today's
  name filter matches the union, so refusal breaks working configs.
  Recommendation: lower every id matching a label name to `in` (id-keyed and
  deterministic, today's semantics), or narrow candidates to in-scope teams
  first. Keep refusal for `project` and `assignee`.
- **Grow-path refresh vs simplicity**: keeping the refresh needs a new seam
  (a port method or a Linear-only client in `work-cli`). Dropping it leaves
  `init-linear` (made non-clobbering) or a dedicated refresh action as the only
  path. Recommendation: drop the grow-path refresh and add a non-clobbering
  workspace refresh.

### Findings

#### Critical

None.

#### Major

- 🟡 **Architecture / Code Quality / Correctness / Compatibility**: Pull-path
  refusals collapse into a transient "retry" discovery failure
  **Location**: Phase 3 §2–§3; Desired End State
  `fetch_page` → `.to_string()` → `page_all` `Transient` → `DiscoveryIncomplete`.
  Resolve in `resolve_scope` (also for broadened scopes) and surface a config
  refusal naming the value and remedy.
- 🟡 **Architecture / Compatibility / Correctness**: `work-cli` cannot reach
  `discover_workspace` through `RemoteTracker`
  **Location**: Phase 1 §3
  The step cannot be built as written without port churn or a second client.
- 🟡 **Correctness / Compatibility / Usability**: Workspace-wide `issueLabels`
  makes team-scoped same-name labels falsely ambiguous
  **Location**: Phase 1 §1; Phase 2 §2; Migration Notes
  Working `label: [Bug]` configs refuse permanently.
- 🟡 **Correctness / Compatibility / Usability**: The mandated `init-linear`
  re-run clobbers the grown `teams` array
  **Location**: Migration Notes; Phase 1 §2
  Multi-team repos lose catalogued teams and see an unexpected diff.
- 🟡 **Test Coverage**: Phases are not structured test-first
  **Location**: all phases
  Tests appear only as `cargo test` lines under Success Criteria. There is no
  red step naming the failing test.
- 🟡 **Test Coverage**: `for_client` mapping for the six new variants is
  untested; `exit_codes_parity` only pins constants
  **Location**: Phase 3 §3
  Ambiguous project and label refusals are never asserted.
- 🟡 **Test Coverage**: "Refuses the pull, no issues fetched" is only verified
  manually
  **Location**: Desired End State; Phase 3 Manual Verification
  Needs a port test asserting zero GraphQL hits, and a CLI flow test.
- 🟡 **Test Coverage**: Sync-level grow wiring and its best-effort contract are
  untested
  **Location**: Phase 1 §3
- 🟡 **Test Coverage**: Extending `catalogue.golden.json` conflicts with the
  `discover_team` golden assertion
  **Location**: Phase 1 §4
  Use a separate `workspace.golden.json`.
- 🟡 **Test Coverage / Usability**: Third `compose` call site (`run_search`)
  and the interactive `search` behaviour change are unaddressed
  **Location**: Phase 3 §2; What We're NOT Doing
  The `search-linear-issues` and `init-linear` skill docs also go stale.
- 🟡 **Code Quality / Architecture**: Six near-identical error variants and
  parallel traits/outcomes; filter family is implicit
  **Location**: Phase 2 §1; Phase 3 §3
  Model `FilterFamily` with one resolver trait and one or two error variants.
- 🟡 **Code Quality / Architecture**: Workspace sections are an untyped
  `Value` with keys repeated as literals in four places
  **Location**: Phase 1 §1–§3; Phase 2 §2
- 🟡 **Usability**: The only recovery path for a stale catalogue is a heavy,
  interactive, clobbering re-init
  **Location**: Phase 3 §3; Migration Notes
  Routine new labels and teammates now require full re-setup.

#### Minor

- 🔵 **Code Quality / Architecture**: Silent `_ => {}` intake catch-all kept;
  accepted keys defined in two unlinked crates — Phase 3 §2
- 🔵 **Correctness / Compatibility / Usability**: Default connection args drop
  archived projects/labels and deactivated users — Phase 1 §1
- 🔵 **Correctness / Usability**: "Or a team-adding pull" migration route is
  unreachable with a filter configured — Migration Notes
- 🔵 **Code Quality / Correctness**: Grow-path design left undecided; `Option`
  flag argument; partial-failure semantics unspecified — Phase 1 §3
- 🔵 **Compatibility**: New exit code misuses the captured-code parity fixture
  (count pin 56), and the 75–79 search block is full — Phase 3 §3
- 🔵 **Compatibility / Code Quality**: Unlisted consumers of `LinearClient::new`,
  `Search`, `compose` (`context.rs:213`, `main.rs:103/113`, test supports);
  client grows to five `Box<dyn>` fields — Phase 3 §2
- 🔵 **Architecture / Usability**: `SEARCH_BAD_STATE` now means any unknown
  value; no `E_*` identifiers specified — Phase 3 §3
- 🔵 **Architecture**: `accepted_filter_keys` bypasses and orphans the public
  `FilterSchema` type — Phase 4 §1
- 🔵 **Architecture / Compatibility**: The committed `catalogue.json` becomes a
  churny workspace-wide snapshot (with emails); older binaries strip the new
  sections — Phase 1; Migration Notes
- 🔵 **Compatibility**: Init stdout envelope grows to the whole workspace,
  flooding the skill's context — Phase 1 §2
- 🔵 **Usability**: A missing catalogue section is reported the same way as a
  typo'd value — Phase 3 §3
- 🔵 **Usability**: Refusal stops at the first unresolved value — Phase 3 §1
- 🔵 **Code Quality**: Existing doc comments go stale (`Search`, `compose`,
  `FILTER_SCHEMA`) — Phase 3 §1; Phase 4 §1
- 🔵 **Test Coverage**: User-resolver edge cases (precedence both ways, higher
  tier ambiguity blocking, null/empty fields, missing sections) not enumerated
  — Phase 2 §2
- 🔵 **Test Coverage**: Discovery page-2 failure, cursor threading, and init
  partial-failure not planned — Phase 1 §1
- 🔵 **Test Coverage**: Test-helper changes and the rewrite of the existing
  `labels.name.in` port assertion are unplanned — Phase 3 §5
- 🔵 **Test Coverage**: `configure`-time Jira rejection only verified manually
  — Phase 4 §2
- 🔵 **Correctness / Compatibility**: ASCII-only `normalise` narrows today's
  server-side `eqIgnoreCase` — Phase 2 §2
- 🔵 **Performance**: Resolution re-runs on every search page — Phase 3 §2
- 🔵 **Performance**: Resolver index structure unspecified; the template is a
  linear scan that allocates — Phase 2 §2
- 🔵 **Performance**: `catalogue.json` read and parsed once per resolver on
  every command — Phase 3 §2
- 🔵 **Performance**: Unbounded serial pagination over potentially large
  connections — Phase 1 §1

#### Suggestions

- 🔵 **Performance**: Quantify the added discovery cost against Linear's hourly
  budgets — Performance Considerations
- 🔵 **Usability**: Log each name→id resolution next to the filter echo —
  Phase 3 §1
- 🔵 **Usability**: Hint `additional_projects` when Jira rejects `project` —
  Phase 4 §1
- 🔵 **Correctness**: Confirm `displayName` uniqueness and pin the tier-order
  decision with a test — Phase 2 §2

### Strengths

- ✅ Phase ordering removes any silent-drop window: `project` is lowered and
  intake-wired before config validation accepts it.
- ✅ Id comparators (`project.id`, `labels.id` with implicit "some",
  `assignee.id`) match the live-verified `IssueFilter` shapes. Fixture rows use
  real-shaped UUIDs.
- ✅ Tier precedence lives in a pure `CatalogueUsers` resolver behind
  `UserResolution`, naming the deciding `MatchTier`.
- ✅ The `Resolvers` bundle changes `compose`'s call sites and fixture builder
  once. Resolvers stay abstract behind traits with `Fixed*` doubles.
- ✅ The shared `paginate` helper keeps the fail-loud-on-any-page contract
  with explicit `first: 250` and scalar-only selections.
- ✅ Config remains human-readable (names, emails). Refusal replaces silent
  wrong-set filtering, matching the familiar `state` behaviour.
- ✅ Deleting `ignore_case_comparator` removes an exception and fixes the
  latent case-sensitive multi-value assignee `in`.
- ✅ Per-tracker keys live on the existing `Tracker` enum inside the private
  `tracker-support` surface.

### Recommended Changes

1. **Move filter resolution to the pre-flight scope step** (addresses: pull-path
   refusals collapse into transient; resolution re-runs per page; "refuse,
   fetch nothing" untested)
   Resolve `project`/`label`/`assignee` in `LinearClient::resolve_scope` (and
   for broadened scopes), carry the ids in the scope, and have `compose` only
   lower ids. Add a `cli_sync` test that asserts an unknown label refuses as
   `DiscoveryUnconfigured` naming the value and `init-linear`. Add a port test
   asserting zero GraphQL hits.
2. **Decide the grow-path seam, or drop the grow refresh** (addresses:
   `work-cli` cannot reach `discover_workspace`; grow design undecided;
   untested grow wiring)
   Recommended: drop it and state `init-linear` as the only refresh. If kept,
   name the seam, specify that a fetch failure warns while the team append
   commits, and test all three branches.
3. **Make init preserve `teams`** (addresses: re-init clobbers `teams`; heavy
   recovery path; "or a team-adding pull" unreachable)
   Read-merge-write under the lock. Correct the Migration Notes to name
   `init-linear` as the sole remedy.
4. **Resolve label names to the set of matching ids** (addresses: false label
   ambiguity)
   Lower all ids for a name into `in`, or narrow to in-scope teams. Add a
   two-teams-each-with-`Bug` test.
5. **Decide archived/disabled inclusion** (addresses: default connection args)
   Pass `includeArchived`/`includeDisabled`, or document the narrowing and
   word the NotFound message to cover it.
6. **Restructure every phase as red→green steps** (addresses: not test-first;
   untested `for_client`; resolver edge cases; discovery error paths; golden
   conflict)
   Name each failing test before the production change. Add a table-driven
   `for_client` test, a separate `workspace.golden.json`, the enumerated
   resolver edge cases, and discovery page-2 failure and cursor tests.
7. **Model the filter family explicitly** (addresses: six variants and parallel
   traits; `SEARCH_BAD_STATE` overload; exit-code fixture misuse; untyped
   sections)
   Introduce `FilterFamily`, one name-resolver trait and one `Resolution` with
   an optional tier, and `UnknownFilterValue`/`AmbiguousFilterValue`. Introduce
   a `WorkspaceCatalogue` type and a `Catalogue` parsed once in `from_config`.
   Pin the new exit code with a dedicated test, not the captured fixture.
8. **Inventory every affected consumer** (addresses: third `compose` call site;
   unlisted `LinearClient::new` callers; stale skill docs and doc comments)
   Add `run_search`, `build_with_override`, the test supports, and the
   `search-linear-issues`/`init-linear` skill docs to Phase 3. Remove the stale
   comments.
9. **Make the intake exhaustive** (addresses: silent `_ => {}`)
   Refuse an unknown key, or add a test that every Linear-accepted key reaches
   `Search`.

## Per-Lens Results

### Architecture

**Summary**: Layering is mostly sound (pure resolvers, `compose` as a
functional core, correct phase ordering). Two structural problems sit at the
boundaries: resolution bypasses the port's `resolve_scope` pre-flight seam, and
the grow refresh assumes `work-cli` can call a Linear-only method it cannot
reach. Smaller issues are scattered catalogue-shape knowledge, a retained
silent-drop intake, and per-family duplication.

**Strengths**: pure, I/O-free resolvers with tier logic in `CatalogueUsers`;
the `Resolvers` bundle; no silent-drop window; per-tracker keys on `Tracker`;
an explicit migration tradeoff grounded in 0220.

**Findings**:
- 🟡 major / high — Phase 3 §1, §3 — *Filter resolution bypasses the port's
  pre-flight resolve_scope seam, so sync pull refusals surface as transient.*
  A `compose` error becomes a `Transient` read and then `DiscoveryIncomplete`.
  The entity and remedy only reach a `tracing::warn`. Resolve in `resolve_scope`
  (returning `ScopeError`, mapped to `DiscoveryUnconfigured`), including for
  broadened scopes. Add a `cli_sync` test.
- 🟡 major / high — Phase 1 §3 — *work-cli cannot reach discover_workspace
  through the RemoteTracker port.* The options are a port extension (snapshot
  churn), a downcast, or a second client. Decide the seam, or drop the grow
  refresh.
- 🔵 minor / high — Phase 3 §2 — *Silent-drop catch-all intake retained;
  accepted keys defined in two crates.* Refuse unknown keys, or add a
  cross-crate test.
- 🔵 minor / medium — Phase 2 §1; Phase 3 §3 — *Per-family duplicate traits
  and six near-identical error variants make the family an implicit axis.*
  Introduce `FilterFamily`, one trait, and `UnresolvedFilter { family, value,
  reason }`. Move state onto `Resolution`.
- 🔵 minor / high — Phase 4 §1 — *accepted_filter_keys bypasses the
  FilterSchema domain type and orphans it.* Return `FilterSchema` per tracker.
- 🔵 minor / medium — Phase 1 §2–§3; Phase 2 §2 — *Catalogue document shape
  knowledge scattered across four sites.* Add a single `Catalogue` aggregate
  loaded once.
- 🔵 minor / medium — Phase 1; Migration Notes — *Committed catalogue.json
  shifts from team-scoped config to a workspace-wide snapshot without
  acknowledgement.* State the churn and conflict tradeoff, or split the file.
- 🔵 suggestion / medium — Phase 3 §3 — *SEARCH_BAD_STATE is repurposed for
  unknown project/label/assignee.* Add `SEARCH_FILTER_UNKNOWN`, or record why.

### Code Quality

**Summary**: Follows the `state` template with well-scoped, testable phases.
Maintainability risks: refusals are stringified and treated as transient on the
pull path, near-identical types proliferate, and workspace sections are an
untyped `Value` with string-literal keys. The grow-path design is undecided,
and the silent catch-all is kept.

**Strengths**: the `Resolvers` bundle; pure tier resolver naming `MatchTier`;
the shared `paginate`; deleting `ignore_case_comparator`; per-tracker keys on
`Tracker`.

**Findings**:
- 🟡 major / high — Phase 3 §2 — *Refusal errors are flattened to strings and
  treated as transient on the pull path.* Compose once before `page_all`, map
  to a non-retryable `TrackerError`, and add a pull-path test.
- 🟡 major / medium — Phase 3 §3 — *Six near-identical error variants, three
  mapped to a state-named code.* Use `FilterFamily` plus
  `UnknownFilterValue`/`AmbiguousFilterValue`. Rename or alias the code.
- 🟡 major / medium — Phase 2 §1 — *Parallel resolver traits and outcome enums
  duplicate one concept.* Use one `NamedEntityResolver` / generic
  `CatalogueNamed`, and parse the catalogue once into a `Catalogue`.
- 🟡 major / medium — Phase 1 §1–§3 — *Workspace sections passed as untyped
  Value with keys as repeated string literals.* Introduce a
  `WorkspaceCatalogue` type.
- 🔵 minor / high — Phase 1 §3 — *Grow-path design left undecided; Option
  parameter conflates two responsibilities.* Prefer a separate
  `refresh_workspace`, and state that the team append survives a fetch
  failure.
- 🔵 minor / high — Phase 3 §2 — *The silent `_ => {}` catch-all is kept.*
  Fail loud, or parse into a typed `FilterKey`.
- 🔵 minor / medium — Phase 3 §2 — *Client grows to five Box<dyn> fields and
  a five-resolver constructor.* Own a `CatalogueResolvers` exposing
  `as_resolvers()`.
- 🔵 minor / medium — Phase 3 §1; Phase 4 §1 — *Existing doc comments will
  go stale.* Remove or rewrite the `Search`, `compose` and `FILTER_SCHEMA`
  docs, and drop the unused `FilterSchema` import.

### Test Coverage

**Summary**: A test anchor exists for nearly every layer at the right level,
but the phases are not test-first. The core refusal property is manual only,
the `for_client` mapping and the sync grow wiring are untested, and planned
edits collide with existing assertions.

**Strengths**: the golden fixture extended under the family guard; a port test
closing the `_ => {}` drop; distinct fixture ids per entity type; the pure tier
resolver with doubles; later-page discovery entities via `Route::Sequence`;
Phase 4 mirroring the fail-before-discovery pattern.

**Findings**:
- 🟡 major / high — all phases — *Phases are not structured test-first.* List
  failing tests by name and assertion before each change.
- 🟡 major / high — Phase 3 §3 — *ClientError→exit-code mapping untested;
  exit_codes_parity only pins constants.* Add a table-driven `for_client` test
  and ambiguous project/label compose tests.
- 🟡 major / high — Desired End State — *"Refuses the pull, no issues fetched"
  only verified manually.* Add a port test (zero GraphQL hits) and a CLI flow
  test.
- 🟡 major / high — Phase 1 §3 — *Sync-level grow wiring and best-effort
  contract have no test.* Cover refresh on new team, none otherwise, and
  failure warns while the team append commits. Add `workspace: None` cases.
- 🟡 major / high — Phase 1 §4 — *Extending catalogue.golden.json conflicts
  with the discover_team golden assertion.* Add a separate
  `workspace.golden.json`, and pin the merged doc in `flow_init`.
- 🟡 major / medium — Phase 3 §2 — *Third compose call site (interactive
  search) untested.* Add `flow_search` scenarios for `--label` and
  `--assignee`.
- 🔵 minor / medium — Phase 2 §2 — *User-resolver edge cases not enumerated.*
  Cover precedence both ways, higher-tier ambiguity blocking, normalisation,
  null/empty fields, and missing sections.
- 🔵 minor / medium — Phase 1 §1 — *Discovery error paths and cursor threading
  unplanned.* Add page-2 failure, cursor propagation, and init
  partial-failure tests.
- 🔵 minor / medium — Phase 3 §5 — *Test-helper and existing-assertion changes
  unplanned.* Add a single fixed-bundle helper, and rewrite `labels.name.in` to
  ids.
- 🔵 minor / low — Phase 4 §2 — *configure-time rejection only verified
  manually.* Optionally add a launcher config test and an end-to-end
  `project.id` scenario.

### Correctness

**Summary**: The core lowering is sound. Edge problems: pull-path refusals
become "transient, retry"; workspace-wide `issueLabels` creates false ambiguity;
the mandated re-init clobbers `teams`.

**Strengths**: refusal closes the silent wrong-set failure; `comparator` is a
correct replacement and fixes the latent multi-value assignee bug; the tier walk
is deterministic; fail-loud pagination prevents truncated catalogues.

**Findings**:
- 🟡 major / high — Phase 3 §3 — *Pull-path refusals collapse into "transient
  cutoff, retry".* Resolve in `resolve_scope`, or add a non-transient outcome,
  and add a `cli_sync` test.
- 🟡 major / medium — Phase 1 §1; Phase 2 §2 — *Workspace-wide issueLabels
  makes team-scoped same-name labels falsely ambiguous.* Fetch `team { id }`
  and narrow, or lower all matching ids to `in`.
- 🟡 major / high — Migration Notes; Phase 1 §2 — *Mandated re-init clobbers
  the grown teams array.* Read-merge-write init, or add a workspace-only
  refresh.
- 🔵 minor / high — Migration Notes — *"Or a team-adding pull" route is
  unreachable with a filter configured.* Drop the clause.
- 🔵 minor / high — Phase 1 §3 — *Grow-path wiring and partial-failure
  semantics unspecified.* Name the route, and on failure call
  `grow_catalogue(&entries, None)`.
- 🔵 minor / medium — Phase 1 §1 — *Default args drop deactivated users and
  archived projects/labels.* Decide explicitly.
- 🔵 minor / high — Phase 3 §2 — *Third compose call site in run_search
  unaccounted for.* Add a `resolvers()` accessor.
- 🔵 minor / low — Phase 2 §2 — *ASCII-only case folding narrows versus
  server eqIgnoreCase.* Use `to_lowercase()`.
- 🔵 suggestion / low — Phase 2 §2 — *A non-unique full-name tier outranks a
  possibly-unique display-name tier.* Verify, and pin with a test.

### Compatibility

**Summary**: Wire shapes are sound, and catalogue growth is additive and
round-trips on older binaries. The contracts the plan gets wrong are the sync
refusal path, the snapshotted `RemoteTracker` port for the grow refresh, label
semantics for multi-team names, the `teams`-clobbering migration, and the new
exit code's placement.

**Strengths**: no silent-drop release window; additive catalogue with
unknown-key round-trip; private `FILTER_SCHEMA` and the `linear-client`
exemption keep snapshots clean; live-verified id forms and real UUIDs; explicit
`first: 250`.

**Findings**:
- 🟡 major / high — Phase 3 §3 — *In the sync path a filter-resolution refusal
  becomes a "transient" read, not the promised refusal.* Resolve in
  `resolve_scope`, and add a `cli_sync` scenario.
- 🟡 major / high — Phase 1 §3; Key Discoveries — *discover_workspace from the
  grow path needs a snapshotted `tracker` API change.* Pick the mechanism, and
  qualify the no-churn claim.
- 🟡 major / medium — Phase 2 §2; Migration Notes — *Label names duplicated
  across teams now refuse permanently.* Lower all matching ids to `in`, or
  narrow to scope, and correct the Migration Notes.
- 🟡 major / high — Migration Notes; Phase 1 §2 — *The migration step wipes
  the grown teams array.* Make init merge, or add `init refresh-workspace`.
- 🔵 minor / medium — Phase 1 §1 — *Default connection filters exclude
  deactivated users and archived projects/labels.*
- 🔵 minor / high — Phase 3 §3 — *New exit code misuses the captured-code
  parity fixture and the search block has no free slot.* Pin with a dedicated
  test, and document band placement.
- 🔵 minor / high — Phase 3 §1–§2 — *Unlisted consumers of LinearClient::new /
  Search / compose.* Cover `main.rs:103/113`, `context.rs:213`, and
  `sync_run_real_client.rs:190`.
- 🔵 minor / medium — Phase 1 §2 — *Init stdout envelope grows to the whole
  workspace, including emails.* Print the team-only shape plus counts.
- 🔵 minor / low — Migration Notes — *Teammates on older binaries strip the new
  sections from the committed catalogue.*
- 🔵 suggestion / low — Phase 2 §2 — *ASCII-only case folding narrows
  server-side case-insensitive assignee matching.*

### Usability

**Summary**: Config stays human-readable, and refusal matches the familiar
`state` behaviour. The recovery path is the main DX problem: a heavy,
interactive, clobbering re-init is the one remedy even for routine staleness,
label and project ambiguity has no config-level fix, and the interactive
`search` change and its skill doc go unacknowledged.

**Strengths**: human-readable config with no syntax break; loud refusals; the
ambiguity message names the tier; Jira rejection reuses the clear
`UnsupportedFilterKey`; no silent-drop window.

**Findings**:
- 🟡 major / medium — Phase 2; Phase 3 §1 — *Label/project ambiguity is a dead
  end with no config-level fix.* Lower all matching label ids to `in`, or offer
  qualification, and list candidates in the error.
- 🟡 major / high — Phase 3 §3; Migration Notes — *The only recovery path is a
  heavy, interactive, clobbering re-init.* Print a non-interactive command, or
  add a workspace-only refresh, and preserve `teams`.
- 🟡 major / high — Phase 3 §1; What We're NOT Doing — *The change to the
  interactive `search` subcommand and its skill is not acknowledged.* Update
  `search-linear-issues/SKILL.md` and `init-linear/SKILL.md`.
- 🔵 minor / high — Migration Notes — *"Or a team-adding pull" remedy is
  circular.*
- 🔵 minor / medium — Phase 3 §3 — *A missing catalogue section is reported
  the same way as a typo'd value.* Add a missing-section outcome.
- 🔵 minor / medium — Phase 1 §1; Phase 3 §3 — *"Re-run init-linear" is wrong
  for archived/deactivated entities.*
- 🔵 minor / medium — Phase 3 §3 — *Unknown label/assignee/project share the
  "bad state" exit code; no `E_*` identifiers specified.*
- 🔵 minor / medium — Phase 3 §1 — *Refusal stops at the first unresolved
  value.* Report all of them together.
- 🔵 suggestion / medium — Phase 3 §1 — *Make name→id resolution visible for
  debugging.*
- 🔵 suggestion / medium — Phase 4 §1 — *Point Jira operators to
  `additional_projects` when they write a `project` filter.*

### Performance

**Summary**: A modest footprint: flat workspace connections, scalar
selections, `first: 250`, and local resolution. The gaps are client-side:
per-page re-resolution, an unspecified (linear-scan) index structure, five
parses of a larger catalogue per command, and unbounded pagination with no cost
estimate.

**Strengths**: flat, scalar, `first: 250` discovery queries; local resolution at
equal filter cost; a single `paginate` helper; grow refresh only when a team is
added.

**Findings**:
- 🔵 minor / high — Phase 3 §2 — *Filter resolution re-runs on every search
  page.* Compose once before `page_all`.
- 🔵 minor / medium — Phase 2 §2 — *Resolver index structure unspecified;
  CatalogueStates template is a linear scan.* Use a pre-normalised
  `HashMap<String, Vec<String>>`.
- 🔵 minor / high — Phase 3 §2 — *catalogue.json read and parsed once per
  resolver on every Linear command.* Parse once, and build lazily.
- 🔵 minor / medium — Phase 1 §1 — *Unbounded serial pagination over
  potentially large connections.* Bound with `discovery_max_pages`, failing
  loud.
- 🔵 suggestion / medium — Performance Considerations — *Quantify the added
  discovery cost against Linear's rate limits.*

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-23

**Verdict:** REVISE

All 60 first-pass findings were re-checked. 51 are resolved, 8 are partially
resolved, and 1 is still present. The structural fixes hold, as all seven lenses
agree: filters now resolve in the pre-flight `resolve_scope`; label names keep
their union semantics; `init` preserves `teams`; archived and disabled entities
are catalogued; and the domain is modelled explicitly. The verdict stays REVISE
because the edits introduced five new major findings. Most are about wiring
detail and test harness feasibility, not the design.

### Previously Identified Issues

- 🟡 **Architecture / Code Quality / Correctness / Compatibility**: Pull-path
  refusals collapse into a transient failure — Resolved
- 🟡 **Architecture / Compatibility / Correctness**: `work-cli` cannot reach
  `discover_workspace` — Resolved (grow-path refresh dropped)
- 🟡 **Correctness / Compatibility / Usability**: False label ambiguity —
  Resolved (union of matching ids)
- 🟡 **Correctness / Compatibility / Usability**: Re-init clobbers `teams` —
  Resolved
- 🟡 **Test Coverage**: Phases not test-first — Resolved
- 🟡 **Test Coverage**: `for_client` mapping untested — Resolved
- 🟡 **Test Coverage**: "Refuse, fetch nothing" manual only — Partially
  resolved (port and `flow_search` assert zero hits; the sync-level tests
  cannot run where they were placed)
- 🟡 **Test Coverage**: Grow wiring untested — Resolved (behaviour removed)
- 🟡 **Test Coverage**: Golden conflict — Resolved
- 🟡 **Test Coverage / Usability**: `run_search` and skill docs — Resolved
- 🟡 **Code Quality / Architecture**: Per-family duplication — Partially
  resolved (`StateResolver` survives beside `NameResolver`; `Unresolved`
  duplicates `Resolution`)
- 🟡 **Code Quality / Architecture**: Untyped workspace sections — Partially
  resolved (typed read model, untyped write model, two encodings of the
  section shape)
- 🟡 **Usability**: Heavy recovery path — Partially resolved (non-clobbering;
  the lighter refresh is out of scope by decision; the remedy could print the
  non-interactive command)
- 🔵 **Usability**: Project ambiguity dead end — Partially resolved (labels
  fixed; projects still say only "rename one")
- 🔵 **Compatibility**: Exit-code fixture misuse — Partially resolved (fixture
  left alone, but 139 is 128+SIGSEGV and `SEARCH_NO_CATALOGUE` (77) is unused)
- 🔵 **Code Quality**: Stale doc comments — Partially resolved (the inline
  comment in `resolve_scope` and the `catalogue.rs` module doc were missed)
- 🔵 **Usability**: Resolution invisible — Still present (accepted as out of
  scope)
- 🔵 All remaining first-pass minors and suggestions — Resolved

### New Issues Introduced

Major:

- 🟡 **Code Quality / Architecture / Correctness**: The concrete
  `CatalogueResolvers` removes the injection seam.
  - `LinearClient::new` takes a concrete, catalogue-backed bundle, so the
    planned `client_with_resolvers(FixedNames…)` helper cannot be built.
  - Removing `states()` breaks `transition.rs:28` (`resolve_all`, exit codes
    122/123) and its `StateResolver` doubles.
  - `fetch_all`'s `page_all` caller is not listed.
- 🟡 **Test Coverage / Correctness / Architecture**: Nothing proves that
  resolved ids reach the search on a broadened pull.
  - `run.rs` must pass `resolve_scope`'s *output* to `resolve_entities`.
  - `RecordingTracker::resolve_scope` is an identity, so a guard-only call
    passes every planned test.
  - Only one broadened `resolve_scope` shape is tested, the adapter re-derives
    `is_broadened`, and `Keyed{base: None, additional}` must not raise
    `E_SEARCH_NO_TEAM`.
- 🟡 **Test Coverage**: The planned `cli_sync` discovery tests cannot run.
  - Verified: the suite calls `scrub_provider_env` and has no `MockServer`, so
    Linear exits 74 before `resolve_scope`.
  - The unknown-label, pre-upgrade and `project.id` end-to-end tests belong in
    `work-adapters/tests/sync_run_real_client.rs`.
- 🟡 **Usability**: The sync surface still frames the migration refusal as
  generic trouble.
  - The refusal prints as `refused: discovery is unconfigured` with exit 74.
  - `sync-work-items/SKILL.md` treats 74 as "fix your config" or
    `--push-only`, and never mentions `init-linear`.
- 🟡 **Code Quality**: Filters round-trip through untyped `(field, value)`
  pairs whose meaning flips from names to ids.
  - The fan-out of one label to many ids is unspecified.
  - `FilterKey` parsing is duplicated across two call sites.

Minor:

- 🔵 **Performance / Compatibility / Correctness**: Catalogue and team
  pagination borrow the operator-tunable `discovery_max_pages`. `init` can fail
  for operators who use none of the new filters, and `enumerate_visible_entities`
  loses its "to exhaustion" contract.
- 🔵 **Compatibility / Correctness**: Exit 139 reads as SIGSEGV. Map
  all-`NotCatalogued` to `SEARCH_NO_CATALOGUE` (77).
- 🔵 **Correctness / Usability**: Including archived projects makes reused
  names ambiguous. Prefer a single non-archived match (select `archivedAt`).
- 🔵 **Correctness**: No stated behaviour for a malformed `catalogue.json`.
  Degrade a bad section to `NotCatalogued`, and make `refresh_catalogue`
  refuse to merge onto an unparseable file.
- 🔵 **Test Coverage**:
  - Existing tests broken by the change are unlisted (`flow_init`'s
    `team-states-200` scenario, `an_unknown_state_is_refused…`).
  - The `RecordingTracker` extension and the no-enumeration assertion are
    unplanned.
  - `CatalogueTruncated` has no exit mapping and no init test.
  - The parsed-once test asserts construction, not behaviour.
- 🔵 **Usability**:
  - The remedy could print `accelerator linear init discover --team-id <id>`.
  - Step 3 of `init-linear/SKILL.md` is stale.
  - Pull filters (`project`, the assignee tiers) have no operator-facing config
    docs.
- 🔵 **Code Quality**: `WorkspaceCatalogue` has required fields while the read
  side needs `Option` sections, so there are two encodings. Unify them in one
  document type with `#[serde(flatten)]` extras.
- 🔵 **Compatibility**: Note in the Migration Notes that adopting `project`
  requires every teammate on the new plugin.

### Assessment

The design is now sound, and the remaining issues are local. To reach APPROVE:

1. Restore injection by abstraction: a boxed trait-object bundle, and keep a
   state path for `transition.rs`.
2. Make the broadened wiring explicit (`let prepared = resolve_scope(..)?;
   resolve_entities(&prepared)`), with a rewriting double and tests for both
   broadened shapes.
3. Move the sync end-to-end tests to `sync_run_real_client.rs`.
4. Update `sync-work-items/SKILL.md` for the refusal.
5. Specify the id fan-out contract through one `FilterKey`-owned function.

The minors are cheap to fold in during the same edit, in particular: a
dedicated catalogue page ceiling, exit code 77/non-signal code, preferring
non-archived projects, and malformed-catalogue behaviour.

## Re-Review (Pass 3) — 2026-09-23

**Verdict:** COMMENT

Every pass-2 finding was re-checked. All five pass-2 majors are resolved, all
seven lenses agree on that, and they report no critical findings. Two new major
findings remain, which is below the REVISE threshold of three. The plan is
acceptable, but both majors should be addressed before implementation. One of
them is a genuine correctness gap that needs a design decision.

### Previously Identified Issues

- 🟡 **Code Quality / Architecture / Correctness**: The concrete
  `CatalogueResolvers` removes the injection seam — Resolved (a boxed
  `ResolverSet`; transition keeps exit codes 122/123 through an explicit
  mapping)
- 🟡 **Test Coverage / Correctness / Architecture**: Nothing proves that
  resolved ids reach the search on a broadened pull — Resolved (the
  `prepared` wiring, a rewriting double, and the four-shape suite)
- 🟡 **Test Coverage**: The `cli_sync` discovery tests cannot run — Resolved
  (moved to `sync_run_real_client.rs`)
- 🟡 **Usability**: The sync surface frames the refusal as generic — Resolved
- 🟡 **Code Quality**: The untyped name/id round-trip — Resolved (`FilterKey`
  named/resolved keys, one pair per id)
- 🔵 **Performance / Compatibility / Correctness**: A borrowed
  `discovery_max_pages` — Resolved
- 🔵 **Compatibility / Correctness**: Exit 139 and SIGSEGV — Resolved (77, 78
  and 89)
- 🔵 **Correctness / Usability**: Archived project ambiguity — Resolved
- 🔵 **Correctness**: Malformed-catalogue behaviour — Partially resolved (the
  read side and refresh are covered; the grow path is still lossy)
- 🔵 **Test Coverage**: Existing tests broken by the change — Partially
  resolved (the `CatalogueTeam` cases are unlisted)
- 🔵 **Test Coverage**: The `RecordingTracker` extension — Partially resolved
  (`Call::EnumerateVisibleEntities` is missing)
- 🔵 **Compatibility**: Unlisted consumers — Partially resolved (the
  `work-cli` grow path's `CatalogueTeam::load` is missing)
- 🔵 **Usability**: Project ambiguity dead end — Partially resolved (the
  active-over-archived case is fixed; candidates are not listed)
- 🔵 All other pass-2 minors — Resolved

### New Issues Introduced

Major:

- 🟡 **Correctness**: State names resolve only against the init team's states,
  which silently narrows broadened pulls.
  - `catalogue.json` stores only the init team's `workflowStates`.
  - On a base+additional or whole-workspace pull,
    `state: [In Progress]` lowers to that one team's state id, so every other
    team's issues are dropped without a refusal.
  - A base-only pull on a grown team that is not the init team returns zero
    issues.
  - The gap predates this plan, but the plan now routes every shape through
    this resolver and its tests would pass.
  - Fix, a decision for the user: either catalogue workspace
    `workflowStates` with union semantics for filters, or refuse `state` on
    scopes not rooted at the init team.
- 🟡 **Test Coverage**: Phase 4's end-to-end `project` test cannot reach
  `resolve_pull_filters` and is never red.
  - Verified: `resolve_pull_filters` is private to `work-cli`.
  - The `work-adapters` harness takes a hand-built scope, and Phase 3
    already makes the test pass.
  - Fix: move the wire test to Phase 3. Drive Phase 4 from
    `work-cli/tests/sync_resolves_real_client.rs`, modelled on
    `linear_bidirectional_without_a_key_refuses_discovery_and_exits_74`. With
    `project: [Nope]`, the run exits 1 before Phase 4 and 74 with
    `E_SEARCH_UNKNOWN_PROJECT` after it.

Minor:

- 🔵 **Compatibility**: Removing `CatalogueTeam::load` breaks
  `work-cli/src/sync.rs:974` (`grow_linear_catalogue`), which the plan does
  not list, so Phase 2 does not compile as written.
- 🔵 **Code Quality / Architecture / Compatibility / Correctness**: The grow
  path round-trips the typed document lossily.
  - It nulls malformed sections, drops unknown entry fields, and writes
    explicit `null`s.
  - Grow and refresh follow different policies for an unparseable file.
  - Fix: a shared strict load-for-write, raw `Value` sections on the write
    model, and `skip_serializing_if`.
- 🔵 **Code Quality / Correctness**: `Resolved(Vec)` admits empty and
  multi-id results, and `resolve_state` has no arm for them. Make emptiness
  unrepresentable.
- 🔵 **Correctness / Test Coverage / Compatibility**: The outcome for an absent
  `workflowStates` (78 or 77) is unspecified and unpinned. `for_client`
  also stops being `const`.
- 🔵 **Test Coverage**:
  - `Call::EnumerateVisibleEntities` is never added.
  - The production catalogue-to-resolver wiring is tested only through
    doubles.
  - The `<team-id>` remedy branches are unasserted.
  - The `CatalogueTeam` tests are unlisted.
  - The broadened label test needs `Route::Sequence` and a negative
    assertion.
- 🔵 **Usability**:
  - The remedy should say "pull the committed catalogue first".
  - An ambiguous project should list its candidates.
  - The multi-line refusal needs a header after sync's prefix.
  - The `label`/`assignee` configure docs should move to Phase 3.
- 🔵 **Compatibility**: 89 sits in the show flow's decade; document it as
  borrowed.
- 🔵 **Architecture**: Suggestion only: test that Linear's accepted keys all
  parse as `FilterKey::Named`.
- 🔵 **Performance**: Suggestion only: build the project, label and user
  indices lazily.

### Assessment

The architecture has converged. Three passes resolved every structural finding,
and the remaining minors are specification detail.

The two open majors should be fixed before implementation:

- The state-narrowing gap is a real wrong-issue-set bug on the paths this plan
  widens, and it needs a user decision.
- The Phase 4 test relocation is mechanical.

With those two addressed, the plan is ready to implement.

## Re-Review (Pass 4) — 2026-09-23

**Verdict:** COMMENT

Both pass-3 majors are resolved. State filters now resolve across every team's
states, and Phase 4's end-to-end test now fails first and then passes, as the
code confirms. Nearly every pass-3 minor is resolved too. There are still no
critical findings.

The union semantics introduced by the state decision leave two new majors,
below the REVISE threshold of three. Both have one root cause: state resolution
is not team-aware.

### Previously Identified Issues

- 🟡 **Correctness**: State names resolve only against the init team's states
  — Resolved (`workspaceStates` union, narrowed by the team clause)
- 🟡 **Test Coverage**: The Phase 4 end-to-end test is never red — Resolved.
  It moved to `work-cli/tests/sync_resolves_real_client.rs`, and it was
  verified to exit 1, then 74.
- 🔵 **Compatibility**: `CatalogueTeam::load` breaks `work-cli` — Partially
  resolved. The re-exports at `linear-client/src/lib.rs:26-27` and
  `tests/cache.rs:272` are unlisted, but the compiler catches them.
- 🔵 **Code Quality / Architecture / Compatibility / Correctness**: The grow
  path is lossy — Partially resolved. Sections are raw `Value` with a strict
  shared loader. However, "every byte survives" is overstated: without
  `preserve_order`, keys come back sorted, and the struct order moves
  `teams`.
- 🔵 **Code Quality / Correctness**: `Resolved(Vec)` can be empty — Resolved
  at the resolver boundary (`ResolvedIds`). The guarantee lapses in
  `ResolvedSearch`.
- 🔵 **Correctness / Test Coverage / Compatibility**: The absent
  `workflowStates` outcome — Resolved (77 for search, 122 for transition,
  pinned)
- 🔵 **Test Coverage**:
  - `EnumerateVisibleEntities` — Resolved
  - Production wiring — Resolved
  - Remedy branches — Resolved
  - `CatalogueTeam` tests — Resolved
  - `Route::Sequence` — Resolved
- 🔵 **Usability**:
  - Pull the committed catalogue first — Resolved
  - Project candidates — Resolved
  - Refusal header — Resolved
  - Configure docs in Phase 3 — Resolved
- 🔵 **Compatibility**: Document 89 as borrowed — Resolved
- 🔵 **Architecture**: Vocabulary-link test — Partially resolved (it checks
  only the named → accepted direction)
- 🔵 **Performance**: Lazy indices — Resolved

### New Issues Introduced

Major:

- 🟡 **Architecture / Correctness**: A team created after the last init is
  silently dropped from state-filtered pulls.
  - `workspaceStates` is an init-time snapshot, and a shared state name still
    resolves through the other teams.
  - The team clause includes the new team, while the state clause excludes
    every one of its states.
  - It never self-heals: grow adds only teams items were imported from, and
    the filter guarantees there are none.
  - Team-scoped labels have the same partial exposure.
  - Fix: when a scoped team has no `workspaceStates` entries, refuse as
    `NotCatalogued`. Keyed scopes can be checked in `resolve_scope`;
    whole-workspace team ids are known only after enumeration.
- 🟡 **Compatibility**: The union silently widens the standalone
  `accelerator linear search --state` from the init team to the whole
  workspace.
  - `run_search` sets no team clause (verified: `team_id: None`,
    `main.rs:103-110`), so only the init-team state id scoped it.
  - This contradicts the search skill's "single-team scoping" contract.
  - Fix: scope `run_search` to the catalogued base team, and pin the result
    with `flow_search`.

Minor:

- 🔵 **Usability / Compatibility**: A state name that the scope's own team
  lacks now yields an empty pull instead of `E_SEARCH_UNKNOWN_STATE`.
  Intersect the union with the scope's teams, and give `NotFound` when the
  intersection is empty.
- 🔵 **Usability**: The remedy's `--team-id` can re-point the init team that
  transition uses. Always print the catalogue's `/team/id`.
- 🔵 **Compatibility**: The interim `compose` wiring between Phase 2 and
  Phase 3 is unspecified, and `tests/filter.rs` is not in Phase 2's update
  list. Phase 2 should resolve `state` through `team_states`.
- 🔵 **Compatibility**: Init no longer overwrites a damaged catalogue, and
  `CacheError::Unparseable` has no exit mapping (`for_cache` is exhaustive).
  Map it, name the recovery, and split the "damaged" remedy from "outdated".
- 🔵 **Correctness / Code Quality**: `load_for_update` cannot tell an absent
  file from an unreadable one (`Filesystem::read` returns `Option`). Widen
  the seam.
- 🔵 **Correctness**: Non-emptiness lapses in `ResolvedSearch` (blank or empty
  ids reach `compose`). Also, `for_client` maps an empty `unresolved` to
  77.
- 🔵 **Code Quality**:
  - The resolver field `states` reads `workspaceStates`; rename it
    `workspace_states`.
  - The named `Search` still has per-family fields.
  - Team-state ambiguity can take two forms.
- 🔵 **Test Coverage**:
  - Nothing pins which resolver serves filters and which serves transition.
  - The 200-page truncation tests are slow; make the ceiling a parameter.
  - The Phase 4 seed needs `team_key` and `projects`.
  - The `scenario_inventory.rs` count and the `seed_catalogue` extension
    are unlisted.
- 🔵 **Suggestions**:
  - Defer the typed section parse into the `OnceCell` (performance).
  - Gloss `workspaceStates` in the `NotCatalogued` message (usability).
  - Record why `workflowStates` is kept (architecture).
  - Say `WorkspaceSections` reuses the `Catalogued*` types (code quality).
  - Add a CHANGELOG `[Unreleased]` entry (compatibility).

### Assessment

The plan is sound apart from one design gap: state (and label) resolution needs
to know which teams are in scope. A single change addresses both majors and the
empty-pull minor:

1. Intersect the union with the scope's teams.
2. Refuse when a scoped team has no catalogued states.
3. Scope the standalone `search` to the catalogued base team.

Only whole-workspace scopes need a decision, because their team ids are known
only after enumeration. The remaining minors are specification detail.

## Re-Review (Pass 5) — 2026-09-23

**Verdict:** REVISE

Nearly every pass-4 finding is resolved:

- Team-aware resolution restores the single-team refusal.
- The standalone `search` is scoped to the init team.
- The write path is strict, lossless and order-stable.
- The remedy always names `/team/id`.
- The interim `compose` wiring is specified.

The new Phase 4 backfill, and the split between Phase 3 and Phase 4 that it
introduced, bring three new majors, which reaches the REVISE threshold. Several
lenses flagged each one independently. All three concern *where* team-scoped
resolution and persistence happen, not *whether* the approach is sound.

### Previously Identified Issues

- 🟡 **Architecture / Correctness**: A team created after init is silently
  dropped — Partially resolved. Backfill covers uncovered teams, but see the
  new majors.
- 🟡 **Compatibility**: The standalone `search --state` widens — Resolved.
- 🔵 **Usability / Compatibility**: A state the scope's own team lacks yields
  an empty pull — Resolved.
- 🔵 **Usability**: The remedy can re-point the init team — Resolved.
- 🔵 **Compatibility**: The interim `compose` wiring — Resolved.
- 🔵 **Compatibility**: Damaged catalogue exit and recovery — Resolved.
- 🔵 **Correctness / Code Quality**: Absent versus unreadable file — Resolved.
- 🔵 **Correctness**: Non-emptiness in `ResolvedSearch` — Resolved.
- 🔵 **Code Quality**:
  - The `workspace_states` rename, the `Search` keyed by family, and
    `WorkspaceSections` reusing `Catalogued*` — Resolved.
  - The dual forms of ambiguity — Partially resolved (acknowledged and
    pinned).
- 🔵 **Test Coverage**:
  - Which resolver serves filters and which serves transition — Resolved.
  - The ceiling seam, the Phase 5 seed, and the inventory and seed lists —
    Resolved.
- 🔵 **Architecture**: The vocabulary-link test — Resolved (the reverse
  direction fails loud at runtime).
- 🔵 **Compatibility**: CHANGELOG — Partially resolved (entries for Phase 1 and
  Phase 4 are missing).
- 🔵 **Correctness / Compatibility**: The byte claim — Partially resolved
  (flattened extras and `null` sections are edge cases).
- 🔵 **Suggestions**:
  - Lazy typed parse, the `workspaceStates` gloss, and why
    `workflowStates` is kept — Resolved.

### New Issues Introduced

Major:

- 🟡 **Code Quality / Architecture / Test Coverage / Correctness**: No
  component resolves covered teams' ids for whole-workspace scopes.
  - Pre-flight treats whole-workspace as "no teams known yet", so it emits
    only `ValidatedName` pairs.
  - Phase 4 extends ids only for *uncovered* teams.
  - Result: either no clause at all (the pull is unfiltered), or only new
    teams' ids (every covered team is dropped).
  - Phase 3 ships with `pending` ignored entirely, which breaks "no window
    where a filter is silently dropped".
  - Fix: pre-flight only *validates* team-scoped names and always carries
    them. `search` does all per-team resolution after enumeration, over the
    concrete UUIDs: covered teams from the catalogue, uncovered teams via
    backfill.
- 🟡 **Code Quality / Test Coverage / Correctness / Architecture**: A pending
  family that ends with zero ids silently drops its filter clause.
  - `compose` lowers only `ids`, so an empty family vanishes, widening a
    state- or label-filtered pull to every issue in scope.
  - This is reachable, because pre-flight's "catalogued in no team" check
    counts teams outside the scope.
  - `search` can return only `TrackerError::Retryable`, so there is no
    channel for a configuration refusal there.
  - Fix: make the drained state a type (`LowerableSearch`). For a zero-id
    family, either refuse or return an empty `Discovery` without paging, and
    decide the channel.
- 🟡 **Usability / Architecture / Compatibility**: Backfill writes the
  committed `catalogue.json` during `sync --preview`.
  - `prepare_run` runs in both modes (`run.rs:1006-1008`), and the port
    has no preview flag.
  - This breaks the skill's "no local write" contract, pinned by
    `a_previewed_remote_only_target_writes_nothing_yet_confirms_it`, and
    departs from `grow_linear_catalogue`, which persists only at
    finalisation, for applied items.
  - Fix: `work-cli` injects a buffering `CatalogueBackfill` that is
    flushed at finalisation in apply mode only.

Minor:

- 🔵 **Architecture / Correctness**: Pending names are triggered by "unknown
  key" rather than by "uncovered team", so a grown team with no
  `workspaceStates` slips through. Workspace labels also resolve only through
  covered teams, and `team(id).labels` likely omits them.
- 🔵 **Test Coverage / Architecture / Correctness**: A backfill fetch failure
  is specified as both `Retryable` and `Transient`, and the engine treats
  them differently (`Failed` versus `DiscoveryIncomplete`).
- 🔵 **Usability / Performance / Correctness**: A team with more than 250
  labels or states fails permanently, yet is reported as transient.
- 🔵 **Performance**:
  - Backfill makes one serial fetch and one full rewrite per team. Batch it
    through the paginated workspace connections filtered by team.
  - The lock wait of up to 5 minutes sits on the critical path. Use a short
    `LockOptions`, or persist after paging.
  - Note that every pull fetches again until the change is committed.
- 🔵 **Compatibility**:
  - The owned-backfill wiring is unspecified. `LinearCache<'a>` borrows its
    filesystem, and `from_config` lacks the project root. About a dozen call
    sites are unlisted.
  - Tail appends conflict across teammates and CI. Insert in sorted position
    instead.
  - The team-id field shape of backfilled entries is unpinned.
  - The `.gitignore` readers are affected by the `read` widening.
  - The CHANGELOG lacks entries for Phase 1 and Phase 4, and needs a Security
    note for emails.
- 🔵 **Usability**:
  - The backfill notice should use the existing `note:` convention, not
    `tracing`.
  - "Picked up automatically" overpromises.
- 🔵 **Code Quality**:
  - Extract the backfill from `search`.
  - Coverage belongs to the catalogue, not to each resolver.
  - Add a `TeamScopedFamily` type.
  - Use one generic `NonEmpty<T>`.
- 🔵 **Test Coverage**:
  - No log-capture harness exists.
  - The no-refetch test must rebuild the client from disk.
  - The ceiling test targets a crate-private helper.
- 🔵 **Correctness**:
  - The standalone search with no catalogued base team is unspecified.
  - States or labels added later to covered teams are missed in multi-team
    scopes.

### Assessment

The pull-filter core has been stable for two passes: project, assignee, label
and state resolution, pre-flight refusal, the lossless catalogue, and the exit
codes. What is not converging is the team-scoped completion added to handle
teams created after init. Each pass has uncovered a new edge in it.

- **Ownership.** Whole-workspace scopes, zero-id families and later-added
  states are all consequences of splitting one resolution rule across
  pre-flight and `search`.
- **Port limitation.** `search` has no configuration-refusal channel.
- **Side effects.** Persisting from inside a read-only port method brings its
  own preview, locking, merge-conflict and CI problems.

A coherent fix is available: validate in pre-flight, resolve every team once in
`search`, gate on a drained `LowerableSearch` type, and buffer the persist
until apply-mode finalisation. However, it needs a decision on the refusal
channel for `search`, which may mean a change to the `tracker` port.
Alternatively, the team-scoped completion (Phase 4, plus team-aware state
resolution for broadened scopes) could be split into its own work item. 0292
would then ship with the pass-3 behaviour for broadened `state` filters, as a
documented limitation.

## Re-Review (Pass 6) — 2026-09-23

**Verdict:** COMMENT

All three pass-5 majors are resolved, and every lens agrees. Four of the fixes
remove the structural problems from pass 5:

- Pre-flight now only validates names.
- Per-team resolution happens in one step, `complete_for_teams`, in `search`.
- `LowerableSearch` gates what `compose` accepts.
- `TrackerError::Unconfigured` is the read-side refusal channel.

The persist is buffered and runs only in apply mode, on the `Ok(report)`
branch next to `grow_linear_catalogue`, so a run that refuses never writes.
The correctness lens confirmed that Linear accepts `WorkflowStateFilter.team`
and `IssueLabelFilter.team` with `id: { in }`.

One new major remains, below the REVISE threshold: the Phase 4 binary tests
cannot run. Everything else is a minor that sharpens the specification.

### Previously Identified Issues

- 🟡 **No component resolves covered teams' ids for whole-workspace scopes**
  (Code Quality, Architecture, Test Coverage, Correctness) — Resolved.
- 🟡 **A pending family left with zero ids silently drops its clause** (the
  same four lenses) — Resolved.
- 🟡 **Backfill writes `catalogue.json` during `--preview`** (Usability,
  Architecture, Compatibility) — Resolved. The test that pins it is
  infeasible, as reported below.
- 🔵 **Pending names triggered by key rather than coverage, and workspace
  labels** (Architecture, Correctness) — Resolved.
- 🔵 **Fetch failure specified two ways** (Test Coverage, Architecture,
  Correctness) — Resolved.
- 🔵 **Truncation for a team with more than 250 entries reported as
  transient** (Usability, Performance, Correctness) — Resolved.
- 🔵 **Performance** — Resolved:
  - per-team serial fetches, now batched;
  - the lock wait on the critical path, now deferred to finalisation;
  - the refetch cost, now documented.
- 🔵 **Compatibility** — Resolved:
  - tail appends, now inserted in sorted order;
  - the `teamId` field shape, now pinned;
  - the `.gitignore` readers;
  - the CHANGELOG, extended to every phase with a Security note.

  Owned wiring is partially resolved: see the registry finding.
- 🔵 **Usability** — Resolved:
  - the `note:` convention;
  - the "picked up automatically" wording.
- 🔵 **Code Quality** — Resolved:
  - the `TeamScopedFamily` type;
  - a single generic `NonEmpty<T>`;
  - coverage answered by the catalogue.

  Extraction from `search` is partially resolved: the fetch has moved into
  `complete_for_teams`.
- 🔵 **Test Coverage** — Resolved:
  - the log-capture harness, no longer needed;
  - the ceiling test, now in the crate.

  The no-refetch test is partially resolved, because it depends on the same
  infeasible harness.
- 🔵 **Correctness** — Resolved:
  - standalone search with no base team (one residual issue below);
  - later-added states, now an explicit non-goal.

### New Issues Introduced

Major:

- 🟡 **Test Coverage: the Phase 4 binary tests assume a loopback override
  that `accelerator-work` does not have.**
  - `ConfiguredTrackers::resolve` calls `from_config` with no endpoint
    override.
  - `ACCELERATOR_LINEAR_API_URL` is read only by `linear-cli`.
  - The module doc of `sync_resolves_real_client.rs` states that
    `from_config` refuses a loopback base.
  - As a result, "apply persists", "preview never writes" and "a failed
    persist warns" cannot be tested as specified.
  - Fix: unit-test `persist_backfilled_sections` inside `work-cli/src/sync.rs`,
    table-driven over apply and preview plus an injected failing write. Move the
    no-refetch check to `sync_run_real_client.rs`.

Minor:

- 🔵 **`TrackerError::Unconfigured` needs a complete integration**
  (Architecture, Code Quality, Compatibility, Correctness).
  - Unlisted exhaustive match sites: `work-cli/src/update.rs:228-246`,
    `create.rs:416-421` (replace `tracker_error_detail` with
    `into_detail`) and `tracker/tests/errors.rs:128-131`.
  - One stated rule is missing. Only `search` produces the variant, and write
    paths treat it as no-remote-change, keeping the baseline. `apply.rs`
    (Retryable) and `for_tracker_error` (74) should agree, or the difference
    should be stated.
  - Rewrite the port's "two classes, and closed" doc for three classes. Update
    the `# Errors` sections on the read methods, the frozen oracle in
    `errors.rs` (74 becomes a third `Class`, and the count becomes 3), the
    `failing_search` doc, and the `work-cli/src/exit_codes.rs` taxonomy.
- 🔵 **Registry wiring does not compose** (Architecture, Compatibility).
  - `with_linear_backfill(Box<…>)` cannot be moved out of `resolve(&self)`.
  - `ConfiguredTrackers::new` is a `const fn` and is built in
    `main.rs:455`, while `sync::run_sync` only receives
    `&dyn TrackerRegistry`.
  - Fix: use an `Arc<dyn CatalogueBackfill>` shared between `main.rs` and
    `run_sync`, and list the changed signature and its callers.
- 🔵 **Keep `complete_for_teams` pure** (Architecture, Code Quality).
  - `search` should split the scoped teams using `covers`, fetch once, call
    `hold`, and then pass the fetched sections in as data (as an overlay
    resolver).
  - Fetched sections must resolve through the same normalisation as the
    catalogued ones.
  - `hold` must run exactly once, after both passes succeed, so a failed
    label pass can never persist states without labels (Correctness).
- 🔵 **Correctness: coverage has no error channel.** A damaged
  `workspaceStates` makes every team look uncovered. Use
  `covers -> Result<bool, CatalogueGap>`, and have `record_team_sections`
  refuse a target that is not an array.
- 🔵 **Test Coverage:**
  - Phase 4 does not list the three Phase 3 uncovered-team refusal tests it
    reverses.
  - The failed-persist test has no fault injection.
  - `apply.rs`'s classification of `Unconfigured` is untested.
- 🔵 **Code Quality:**
  - Hoist the duplicated `resolve_scope` mapping in `run.rs`.
  - `ResolverSet` exposes both `coverage` and `workspace_states`.
  - `NotCatalogued` has no typed slot for the uncovered team.
  - The Phase 4 overview contradicts §3 on how preview runs are wired.
- 🔵 **Usability:** late refusals and the `note:` may name teams by UUID
  rather than key, so carry `(id, key)` through completion.
- 🔵 **Compatibility / Correctness:** `E_SEARCH_NO_TEAM` on standalone
  search has no pinned exit code (77 suggested), and a `--text`-only search
  with no catalogued team is not scoped to the init team.
- 🔵 **Suggestions:**
  - Deduplicate ids during completion (Performance).
  - The truncation remedy should not suggest a refresh that hits the same
    ceiling (Usability).
  - Consider one Linear finalisation writer (Architecture).
  - Add the sync exit-code change from 5 to 74 to the CHANGELOG (Compatibility).

### Assessment

The design has converged. The three structural problems that recurred in
passes 4 and 5 are gone, and no lens raised a new structural concern.

- **Before implementation:** the remaining major is a test-harness correction
  that should be fixed before implementing, using unit tests in
  `work-cli/src/sync.rs` for the apply-only flush.
- **Integration and wiring:** the `TrackerError::Unconfigured` integration
  list and the registry wiring are specification gaps the compiler would
  surface. Fixing them in the plan avoids design decisions being made mid-phase.

## Approval — 2026-09-23

**Verdict:** APPROVE

The reviewer approved the plan after the pass-6 fixes were applied. Those
fixes were:

- the unit-level finalisation tests;
- the full `TrackerError::Unconfigured` integration;
- the `Arc`-based backfill wiring;
- pure completion with a single hold;
- the coverage error channel, plus the listed minors.

No further lens pass was run after these fixes.
