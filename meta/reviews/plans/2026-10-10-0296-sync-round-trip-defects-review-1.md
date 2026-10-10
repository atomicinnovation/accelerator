---
type: "plan-review"
id: "2026-10-10-0296-sync-round-trip-defects-review-1"
title: "Plan Review: Sync Round-Trip Defects Implementation Plan"
date: "2026-10-10T22:05:45+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-10-10-0296-sync-round-trip-defects"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "compatibility", "safety", "standards"]
review_number: 1
review_pass: 8
tags: ["sync", "baseline", "equivalence", "adf", "digest"]
last_updated: "2026-10-12T03:06:40+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Sync Round-Trip Defects Implementation Plan

**Verdict:** REVISE

The plan is well layered and rigorously test-first. The body model copies
the frontmatter port precedent, equivalence-as-canonical-equality removes
hash/predicate drift by construction, and the traceability table makes gaps
auditable. The Phase 8 decision procedure is the weak point: read literally,
it turns one-sided `Unknown` recovery rows into pulls and pushes, can pull an
empty body after a failed `show`, and can push a stale title over a remote
retitle. The Migration Notes' claim about older binaries is also false. An
older binary pushes recipe-1 items and strips the recipe marker from the
whole committed `last-sync.json` on its first write.

### Cross-Cutting Themes

- **Phase 8 rule 3 maps `Unknown` sides to one-sided states** (flagged by:
  correctness, safety) — Empty-hash recovery rows and legacy local changes
  can become pulls or pushes, not the conflicts 0296's recovery table
  requires. Together with the `PushTitle` and `FrontmatterOnly` ambiguities,
  the classification rules need restating as a complete table.
- **Mixed-version `last-sync.json`** (flagged by: architecture,
  compatibility, safety, correctness) — An older binary classifies a
  recipe-1 entry as `LocallyModified` and pushes it, and its four-key
  `render_entry` downgrades every entry on the next write. The
  unknown-recipe discard repeats this hazard for every future recipe.
- **Empty-body pull when the remote body is absent** (flagged by:
  correctness, safety) — `pull_outcome`'s `unwrap_or("")` survives the
  Phase 1/4 rewrite, and `RemoteChange::Unknown`-as-changed keeps the route
  open.
- **Jira dialect spread across `work-adapters`** (flagged by: architecture,
  code-quality) — `Integration` matches appear in six or more functions
  (render, two digests, equivalence, baseline writer, promotion). Tracker
  knowledge leaks past the port that was meant to contain it.
- **Recipe-1 digests depend on unguarded code** (flagged by: architecture,
  compatibility, correctness, test-coverage) — The hash depends on
  `CanonicalBody::render` and the `adf` round trip, yet only the
  `pulldown-cmark` version is guarded. There are no Jira golden bodies, and
  no test that hash equality matches `equivalent`.
- **Unrenderable detection by substring** (flagged by: architecture,
  code-quality, correctness) — Scanning for `[unsupported ADF` couples
  `work-adapters` to placeholder wording and misfires on prose (0296 itself
  quotes it).
- **Exit code 41 left undecided** (flagged by: code-quality, compatibility,
  standards) — Retiring it breaks the frozen `exit_codes_parity` oracle.
- **Port and type shape** (flagged by: code-quality, architecture) —
  `ItemDigests` grows to five methods returning `Option<bool>`, recipe
  variants are named after version numbers, and promotion would have to
  build a file-backed `LazyItemDigests` to reuse `bodies_equivalent`.

### Tradeoff Analysis

- **Architecture (symmetric port) vs scope**: Moving Jira rendering into
  `show_op`, so `RemoteIssue.body` is Markdown for both trackers, keeps
  `work-adapters` tracker-agnostic but changes the tracker contract and
  `jira show`. A lighter option is a single `TrackerBody` dialect value,
  built at the composition root and injected, which keeps the
  `RemoteIssue` shape and confines every `Integration` match to one module.
  Recommended: the dialect value.
- **Forward compatibility vs format simplicity**: Dual-writing a legacy
  `local_hash` (and keeping the legacy `remote_hash` under its old key, with
  the recipe-1 remote hash under a new key) protects older binaries but
  doubles the entry until 0331. The alternative is a documented minimum
  plugin version for anyone sharing `last-sync.json`. Dual-writing is
  safer for a committed, shared file.

### Findings

#### Critical

- 🔴 **Correctness + Safety**: Rule 3 maps a one-sided `Unknown` to a pull
  or push instead of a conflict
  **Location**: Phase 8, §2 Domain
  An empty `remote_hash` with an unchanged local side becomes
  `RemotelyModified` → pull. An empty local hash with an unchanged remote
  becomes `LocallyModified` → push. 0296's recovery table requires an
  unresolved conflict for both not-equivalent rows, so a failed-read-back
  `create --push` with a Linear meaning rewrite would overwrite the local
  file.

#### Major

- 🟡 **Correctness + Safety**: A pull still writes an empty body when the
  remote body was never fetched
  **Location**: Phases 1/4 `pull_outcome`; Phase 8 `RemoteChange::Unknown`
  `unwrap_or("")` survives, and `Unknown` counts as changed, so a transient
  `show` failure can wipe a work item and baseline the wipe.
- 🟡 **Correctness**: A frontmatter-only edit is `Synced` when the stamp
  skips the `show`
  **Location**: Phase 8 §2; Phase 7 §6 `fetch.rs`
  `bodies_equivalent()` is `None` and `FrontmatterOnly` counts as
  unchanged, so local retitles stop propagating, and
  `a_frontmatter_only_edit_is_locally_modified_with_no_update` would fail.
- 🟡 **Correctness + Safety**: `PushTitle` can revert remote title edits
  **Location**: Phase 8 §2 rule 2; §3 `push_outcome`
  Whether `Unknown` or legacy-cosmetic local changes count as "local
  changed" is unspecified, and a remote retitle also makes titles differ.
  This contradicts the recovery rows and the unmarked-entry criterion.
- 🟡 **Safety**: The legacy local-change mapping is unspecified for rule 3
  **Location**: Phase 8 §2
  If a legacy local change maps to `FrontmatterOnly`, then a legacy item
  edited on both sides with non-equivalent bodies pulls over the local edit.
- 🟡 **Compatibility + Architecture + Safety + Correctness**: Older binaries
  push recipe-1 items and downgrade every entry
  **Location**: Migration Notes; Phase 7 §4
  The "reports a conflict" claim is false. `classify.rs:66-99` yields
  `LocallyModified` when the stamp is unchanged, and the four-key
  `render_entry` strips `digest_recipe` and the split hashes from the whole
  document.
- 🟡 **Compatibility + Safety**: Unknown-recipe entries are discarded and
  then deleted on write
  **Location**: Phase 7 §1 `an_unknown_recipe_discards_the_entry_with_degradation`
  A future recipe-2 entry is lost the next time a recipe-1 binary saves.
- 🟡 **Compatibility**: `UnrenderableRemote` has no failure class, exit code
  or report token
  **Location**: Phase 4 §3
  The failure is deterministic, so it does not fit `70` (retryable). The
  skill's exit-code contract would drift.
- 🟡 **Architecture + Code Quality**: Jira body rules are spread as
  `Integration` switches through `work-adapters`
  **Location**: Phases 4, 7 §3, 8 §3, 10 §2
  The `RemoteIssue.body` contract puts projection behind the port. The
  plan instead adds six or more `match integration` sites.
- 🟡 **Architecture + Compatibility + Correctness**: Recipe-1 digests depend
  on three crates, but only the parser version is guarded
  **Location**: Phase 7 `golden_digests.rs`
  Changes to the canonical render or the `adf` converter silently rehash
  every baseline. The guard can also pass if someone edits `recipe.json`
  without bumping the recipe.
- 🟡 **Code Quality + Architecture**: The recipe is modelled three ways with
  near-duplicate enums
  **Location**: Phase 7 §2, §4
  The shapes are `u32` const, undefined `Recipe`, `LocalBaseline<'a>`,
  `LocalDigest`, `LocalDigest | Absent` and `""`. Variants are named after
  version numbers, not meaning.
- 🟡 **Code Quality + Architecture**: The `ItemDigests` port becomes a
  grab-bag with `Option<bool>` answers
  **Location**: Phase 8 §2
  Hashing and semantic comparison are mixed in one port, and `None` versus
  `Some(false)` is decoded by convention.
- 🟡 **Code Quality**: Decision logic is split across classify, decide and
  a plan-level override
  **Location**: Phase 8 §2 `plan.rs`
  Direction handling for `PushTitle`/`PullOnly` is duplicated outside the
  `decide` table.
- 🟡 **Test Coverage**: New ADF fixtures without oracle files break the
  57-case manifest and the differential test
  **Location**: Phase 3 §1
- 🟡 **Test Coverage**: Characterisation tests that pin the removed
  behaviour are not listed for inversion
  **Location**: Phase 3 §5
- 🟡 **Test Coverage**: No fixed-point test that rendered Jira Markdown
  survives the round trip
  **Location**: Phase 3 roundtrip cases
  ADF text starting `> `, `|` or `---`, or containing `~~`, now
  re-assembles as structure.
- 🟡 **Test Coverage**: The Jira pull test uses the function under test as
  its oracle
  **Location**: Phase 4 `jira_pull_writes_rendered_markdown_through_the_real_client`
- 🟡 **Test Coverage**: Nothing tests that hash equality agrees with
  `equivalent` on near-misses
  **Location**: Phases 6/7
- 🟡 **Test Coverage**: Pull entry-point tests miss Jira frontmatter-kept,
  unrenderable `--resolve =remote`, pull-create and dossier
  **Location**: Phases 1 and 4
- 🟡 **Test Coverage**: `RecordBaseline` and the `PushTitle`/`PullOnly`
  override have no plan-layer tests
  **Location**: Phase 7 §6, Phase 8 §2
- 🟡 **Test Coverage**: Recipe-1 writers, realign, pending-push markers and
  recipe-1 blanking are not individually tested
  **Location**: Phase 7 §5
- 🟡 **Test Coverage**: The classify table omits the equivalence-unknown,
  legacy remote-cosmetic and not-equivalent recovery rows
  **Location**: Phase 8 classify table
- 🟡 **Test Coverage**: `RecordingTracker::rewriting` is not anchored to
  the Phase 0 read-back
  **Location**: Phase 8 §3
- 🟡 **Standards**: Package names `work-cli` and `migrate-cli` do not
  exist; the packages are `accelerator-work` and `accelerator-migrate`
  **Location**: Phases 1, 4, 7, 8, 9 verification
- 🟡 **Standards**: `cargo test -p jira-client`/`-p tracker-test-support`
  runs the credential-gated `contract` binary
  **Location**: Phases 2, 3, 8 verification
- 🟡 **Standards**: `mise run test:integration:characterisation` is not a
  task
  **Location**: Phase 3 success criteria
- 🟡 **Standards + Compatibility + Code Quality**: Retiring exit code 41
  breaks the frozen `exit_codes_parity` oracle
  **Location**: Phase 3 §4
- 🟡 **Standards**: `pulldown-cmark` is not confined to `document` with a
  `deny.toml` wrapper ban
  **Location**: Phase 5 §2, §5
- 🟡 **Standards**: The `cli/adf/src/adf/` layout stutters and does not
  resolve from `lib.rs`
  **Location**: Phase 2 §1
- 🟡 **Standards**: The `adf` pup probe pair is never run by Phase 2's checks
  **Location**: Phase 2 success criteria
  `test:integration:pup` is outside `mise run`.

#### Minor

- 🔵 **Architecture + Code Quality + Correctness**: Unrenderable detection
  scrapes the `[unsupported ADF` text
  **Location**: Phase 4 §2
- 🔵 **Architecture + Code Quality**: Promotion reuses `bodies_equivalent`
  from a file-backed digest object
  **Location**: Phases 8 §3, 10 §2
- 🔵 **Architecture**: Wiring for `BodyParser` and `Integration` is
  unspecified
  **Location**: Phases 7 §5, 8 §3, 10 §2
  The risk is a forbidden normal edge from `work-adapters` to
  `corpus-adapters`.
- 🔵 **Architecture**: `DiscoveredIssue` carrying `LocalBody` lets
  `Unrenderable` cross the `LocalAuthor` port, and dropping `RemoteIssue`
  loses the baseline data
  **Location**: Phase 4 §3
- 🔵 **Code Quality**: `LocalChange::Body` hides the frontmatter-and-body
  case; `PushTitle` undersells what it sends
  **Location**: Phase 8 §2
- 🔵 **Code Quality**: `BaselineDigests::of` returns a bare tuple and takes
  a recurring argument clump
  **Location**: Phase 7 §5
- 🔵 **Code Quality**: `digest.rs` and `LazyItemDigests` accrete mixed
  responsibilities
  **Location**: Phases 7 §3, 8 §3
- 🔵 **Compatibility**: In-flight promotion markers carry remote hashes
  whose recipe is not recorded
  **Location**: Phase 7 §5
- 🔵 **Safety**: `RecordBaseline` must not run when the forced `show`
  failed
  **Location**: Phase 7 §6
- 🔵 **Correctness**: `record_baseline` reads "current bodies" at apply
  time
  **Location**: Phase 7 §6
  This can absorb an edit made between classification and apply.
- 🔵 **Correctness**: Classified legacy entries whose action is
  `Noop`/`Prompt`/`SkipDirty` are not rewritten
  **Location**: Phase 7 §6
- 🔵 **Correctness**: The code-span indentation row is only
  CommonMark-distinct inside a container
  **Location**: Phase 6 §1
- 🔵 **Correctness**: Section pairing is by raw heading, and Jira dossiers
  compare the un-round-tripped local body
  **Location**: Phase 9 §2
- 🔵 **Test Coverage**: No test that recipe-1 entries return to the
  stamp-skipped read
  **Location**: Phase 7 §6
- 🔵 **Test Coverage**: Section-splitting edge cases and the `work diff`
  behaviour change are untested
  **Location**: Phases 5, 9
- 🔵 **Test Coverage**: The parser guard's "recipe was bumped" pass path is
  untested
  **Location**: Phase 7
- 🔵 **Standards**: The `record-baseline` keyword is not reflected in the
  skill, and its golden is hedged
  **Location**: Phase 7 §6
- 🔵 **Standards**: The opt-in `tracker-contract` lane is listed as an
  automated gate
  **Location**: Phases 4, 8
- 🔵 **Standards**: Phase 0 gates on `mise run check`, and the
  pending-push test location is unresolved
  **Location**: Phases 0, 7

#### Suggestions

- 🔵 **Code Quality**: Name the table rows/cells and the task marker
  instead of `Vec<Vec<Vec<Inline>>>` and `Option<bool>`
  **Location**: Phase 5 §3
- 🔵 **Code Quality**: The new `Document` in `section_diff` collides with
  existing names
  **Location**: Phase 9 §2
- 🔵 **Compatibility**: Pin only `adf`'s three entry points, not its
  internal modules
  **Location**: Phase 2
- 🔵 **Safety**: Pushes can still overwrite unrenderable Jira content
  **Location**: Phase 4
- 🔵 **Safety**: Allowlist fixture fields, and delete (not archive) the
  scratch issue
  **Location**: Phase 0
- 🔵 **Test Coverage**: Promotion equivalence is tested only for Linear
  **Location**: Phase 10

### Strengths

- ✅ The body model follows the frontmatter split. `corpus` owns meaning,
  `document` owns parsing with no `pulldown_cmark` in either pinned public
  API, an `rg` check enforces that, and `corpus-adapters` bridges them
  through a port.
- ✅ Equivalence is defined as `canonical() == canonical()`, and the hash
  renders the same form, so predicate and digest share one normalisation.
- ✅ Extracting `adf` as its own technical library correctly avoids the
  `remote-projection` → `jira-client` cycle. Registration covers every
  point of the README checklist.
- ✅ Baseline entries become a sum type migrated per entry with no file
  writes or tracker calls, so 0331 can be a deletion.
- ✅ A single `remote_projection::split_projected` returning a named
  `ProjectedIssue` replaces the private splitter.
- ✅ Preview read-only holds structurally (it returns before `ItemApplier`)
  and is pinned by byte-identity tests.
- ✅ Unrenderable Jira ADF fails the item with no write.
- ✅ The exact `pulldown-cmark` pin plus a lock-reading guard follows the
  `queries.rs` precedent, and the guard's failure path is tested.
- ✅ Every phase opens with named failing tests, and the traceability table
  maps nearly every 0296 criterion to a test.
- ✅ Phase 0 captures real Linear read-back and recovery fixtures before
  implementation.
- ✅ Phase 1 keeps the legacy `remote_hash` until Phase 7, so intermediate
  phases classify consistently.

### Recommended Changes

1. **Restate the Phase 8 decision procedure as a complete table**
   (addresses: one-sided `Unknown`; `FrontmatterOnly` with an unfetched
   remote; `PushTitle` reverting remote titles; legacy local-change
   mapping; classify-table omissions)
   - Add a rule ahead of rule 3: either side `Unknown` and not equivalent
     (or equivalence unknown) → `Conflict`.
   - A legacy local change counts as `Body` in rule 3.
   - `PushTitle` fires only on a recipe-1 frontmatter-hash change.
     `Unknown`, legacy and body-only local sides record instead.
   - Force the `show` when a hash is empty or the local frontmatter hash
     differs.
   - Add classify rows for each of these cases.
2. **Never pull without a fetched remote body** (addresses: empty-body
   pull)
   `pull_outcome`/`local_body_of` take `Option<&str>` and fail on `None`.
   A stamp-changed but unfetched remote is `Indeterminate`. Add a
   failed-`show` test.
3. **Make `last-sync.json` forward- and backward-safe** (addresses:
   mixed-version push and downgrade; unknown-recipe discard; Migration
   Notes)
   - Dual-write legacy `local_hash`/`remote_hash` alongside the recipe-1
     keys under new names until 0331.
   - Keep unknown-recipe entries as opaque JSON re-rendered verbatim, and
     classify them `Indeterminate`.
   - Correct the Migration Notes.
4. **Introduce one `TrackerBody` dialect value** (addresses: `Integration`
   spread; promotion reuse; parser wiring; `BaselineDigests` argument
   clump)
   - Built once at the `work-cli` root from `Integration` and `BodyParser`.
   - It owns `local_body_of`, the comparable local and remote forms, and
     `equivalent`, returning a `BodyEquivalence` enum.
   - Digests, pull, dossier and promotion all delegate to it.
5. **Give `adf` a strict render** that returns a typed error for
   unrepresentable nodes (addresses: substring scraping)
6. **Guard every input to the recipe-1 hash** (addresses: unguarded digest
   dependencies; hash/predicate agreement; guard bypass)
   - Make golden rows local/remote pairs, including Jira bodies.
   - Assert that `equivalent` agrees with hash equality on every
     equivalence and near-miss pair.
   - Keep an append-only recipe → parser-version history.
7. **Tidy the domain types** (addresses: recipe modelled three ways;
   `ItemDigests` grab-bag; decision split; naming)
   - One `DigestRecipe`, and `Option<LocalDigest>` named by meaning.
   - `BodyEquivalence`/`TitleComparison` enums.
   - Equivalence resolution as an input to `decide`.
8. **Settle exit code 41** (addresses: exit code 41; failure class for
   `UnrenderableRemote`)
   Keep the constant reserved with no `for_adf` arm, and list the
   `adf_inventory.rs` and node-types edits. Assign `UnrenderableRemote` a
   rejected-class exit and a report token, and document it in the skill.
9. **Fix the verification commands** (addresses: package names, contract
   binary, missing task, pup probes, tracker-contract gate, Phase 0 bar)
   Use `cargo nextest run -p accelerator-work …` and drop
   `test:integration:characterisation`. Add `test:integration:pup` to
   Phase 2, move `tracker-contract` to manual, and gate Phase 0 on
   `mise run`.
10. **Fix the Phase 2/3 crate and fixture plumbing** (addresses: `src/adf/`
    stutter; oracle manifest; characterisation tests; `deny.toml` ban; pin
    surface)
    - Flatten to `cli/adf/src/{tokenise,assemble,render}.rs` and update
      `adf_inventory.rs`.
    - Put the extension fixtures under a separate prefix with their own
      count.
    - List the existing tests to invert.
    - Add the `pulldown-cmark` wrapper ban.
11. **Close the test gaps** (addresses: the remaining test-coverage
    findings)
    - Literal expected Markdown for the Jira pull.
    - A round-trip fixed-point test with syntax-like text.
    - An entry-point × tracker table.
    - `sync_plan.rs` rows for `RecordBaseline`/`PushTitle`.
    - Writer-level recipe-1 assertions.
    - Anchor the rewrite fake to the Phase 0 read-back.
    - Call-count tests for the forced `show`.

## Per-Lens Results

### Architecture

**Summary**: The plan keeps the hexagonal crate rules intact. It confines
`pulldown-cmark` behind `document`, keeps `corpus` serde-free, and mirrors
the frontmatter port split. Equivalence is canonical-form equality. The
main concern is that Jira knowledge moves from the provider client into the
tracker-agnostic sync adapter. Secondary concerns: recipe-1 digests depend
on three crates but only one is guarded, the mixed-version claim
contradicts current classify code, and the dependency wiring is
unspecified.

**Strengths**:
- The Markdown parsing split follows the `FrontmatterParser` precedent, and
  the public APIs stay free of `pulldown_cmark`.
- `equivalent` is defined as canonical equality, and the hash uses the same
  form.
- Extracting `adf` avoids the `remote-projection` → `jira-client` cycle.
- The sum-typed baseline entry migrates per entry, so 0331 is a deletion.
- `split_projected` gets a single owner.
- Preview stays read-only, and classification stays separate from apply.

**Findings**:
- 🟡 major / high — *Phase 4; Phase 7 §3; Phase 8 §3* — Jira-specific
  rendering moves out of the provider client into the tracker-agnostic sync
  adapter. `RemoteIssue.body` (`tracker/src/lib.rs:131`) makes projection
  the client's obligation, and `work-adapters` has no `Integration`
  branches today. Make the port symmetric (Jira `show_op` renders through
  `adf`), or inject a per-tracker `RemoteBodyFormat`, or record the
  trade-off and confine the match to one module.
- 🟡 major / medium — *Phase 7 golden digests* — Recipe-1 digests depend on
  three crates' output, but only the parser version is guarded. Add Jira
  round-trip golden bodies, and consider keeping the hashing serialisation
  in `work-adapters`.
- 🟡 major / medium — *Migration Notes* — The mixed-version claim
  contradicts `classify.rs:71-73`. An old binary pushes, and
  `parse_entry`/`render_entry` drop unknown keys, so the entries churn.
  Choose a forward-compatibility strategy and test it.
- 🔵 minor / high — *Phase 7 §5; 8 §3; 10 §2* — Wiring for `BodyParser`
  and `Integration` is unspecified. Name one injected value built at the
  `work-cli` root.
- 🔵 minor / medium — *Phase 10* — Promotion reuses `LazyItemDigests`
  equivalence. Extract a standalone comparison service.
- 🔵 minor / high — *Phase 4 §2* — Unrenderable detection scrapes
  placeholder text. Add a strict `adf` render entry point.
- 🔵 minor / medium — *Phase 4 §3* — `DiscoveredIssue` carrying
  `LocalBody` lets `Unrenderable` cross the `LocalAuthor` port, and dropping
  `RemoteIssue` loses baseline data. Resolve it in `create_from_remote`.
- 🔵 suggestion / medium — *Phase 7 §2; 8 §2* — Domain types are named
  after persistence versions, and `ItemDigests` is widening. Use
  `WholeFingerprint`/`SplitFingerprint` and split off a comparison port.

### Correctness

**Summary**: The title split, ADF extension, body model and recipe-1
digests are mostly sound and closely traced. Phase 8 rule 3 sends a
one-sided `Unknown`, a `FrontmatterOnly` change and an unfetched remote body
to the wrong states, which contradicts the recovery table. The secondary
issues are the false Migration Notes claim, a bypassable parser guard, and
edge cases in `RecordBaseline` and section pairing.

**Strengths**:
- A single `CanonicalBody` backs both the hash and equivalence.
- Phase 1 keeps the legacy `remote_hash` until Phase 7.
- Legacy entries are classified under their own recipe, and the forced
  `show` closes the stamp-skip gap.
- Preview read-only is structural.
- Unrenderable ADF hashes as `"adf:" + description`, so it is deterministic
  and never equivalent.

**Findings**:
- 🔴 critical / high — *Phase 8 §2* — Rule 3 maps a one-sided `Unknown` to
  a pull or push, not the conflict the recovery table requires. Add an
  explicit `Unknown` + not-equivalent → `Conflict` rule and classify rows.
- 🟡 major / high — *Phase 8 §2; Phase 7 §6* — A frontmatter-only edit
  classifies as `Synced` whenever the stamp skipped the `show`. Force the
  read when the frontmatter hash differs or a hash is empty, or map
  `FrontmatterOnly` + remote unchanged to `LocallyModified`.
- 🟡 major / high — *Phase 8; Phases 1/4 `pull_outcome`* — A failed or
  skipped `show` still leads to a pull of an empty body. Never plan a pull
  without a body, and treat a failed `show` as `Indeterminate`.
- 🟡 major / medium — *Phase 8 §2 rule 2* — "Local changed" and "frontmatter
  changed" are ambiguous for `Unknown` and legacy entries, so `PushTitle`
  can fire unwanted. Base it on a recipe-1 frontmatter-hash change only.
- 🟡 major / high — *Migration Notes* — Older binaries push recipe-1
  entries rather than conflict them, and they strip the new keys. Dual-write
  or gate older binaries out.
- 🔵 minor / medium — *Phase 6 §1* — The code-span indentation row is
  CommonMark-equivalent outside a container. Build the input from the
  read-back's list-item context.
- 🔵 minor / medium — *Phase 7 golden digests* — The parser guard can be
  satisfied without a recipe bump, and ADF changes are unguarded. Keep an
  append-only history and add Jira golden bodies.
- 🔵 minor / medium — *Phase 7 §6* — `record_baseline` reads current bodies
  at apply time. Record the memoised classification hashes, and leave the
  legacy entry alone when no body was gathered.
- 🔵 minor / medium — *Phase 7 §6* — Legacy entries with
  `Noop`/`Prompt`/`SkipDirty` are not rewritten. Specify this as deliberate
  and amend 0296.
- 🔵 minor / high — *Phase 4 §2* — Detecting unsupported nodes by searching
  for `[unsupported ADF` misfires on literal prose. Report unsupported
  nodes structurally.
- 🔵 minor / medium — *Phase 9 §2* — Section pairing by raw heading, and
  Jira comparison against the raw local body, can still list equivalent
  sections. Pair by canonical heading and compare round-tripped sections.

### Test Coverage

**Summary**: The plan is strongly test-first, with a criterion-to-test
table and equivalence and recovery tables covered row by row. The gaps are
in assertion and fixture wiring: the ADF oracle manifest, a self-referential
Jira oracle, no test of hash/predicate agreement, untested plan overrides
and recipe-1 writers, and partial entry-point coverage across both
trackers.

**Strengths**:
- Every phase opens with named failing tests.
- The traceability table maps nearly every criterion.
- The equivalence tests cover cosmetic rows, meaning rows and near-misses
  over real parsing.
- The golden-digest guard has a negative test.
- The recovery tests check follow-up runs and that preview agrees with
  apply.
- Phase 0 captures real fixtures.
- Preview byte-identity is asserted twice.

**Findings**:
- 🟡 major / high — *Phase 3 §1* — New `render-*`/`assemble-*` fixtures
  without oracle files break `adf_oracle_manifest` (pinned at 57) and the
  `frozen_oracle` panic. Use a separate prefix or directory with its own
  count.
- 🟡 major / medium — *Phase 3 §5* — Existing characterisation tests that
  pin the removed behaviour are not listed (`each_of_the_four_refusals…`,
  `the_round_trip_is_asymmetric…`, `the_table_guard_is_narrow_by_design`,
  and others). Rewrite them; do not delete them.
- 🟡 major / high — *Phase 3 roundtrip* — There is no fixed-point test for
  rendered Jira Markdown containing syntax-like text. Add
  `roundtrip(render(adf)) == render(adf)` over all render fixtures, plus
  explicit `> `, `|`, `---` and `~~` text cases.
- 🟡 major / high — *Phase 4* — The Jira pull test uses
  `document_to_markdown` as its own oracle. Assert against literal expected
  Markdown.
- 🟡 major / high — *Phases 6/7* — Nothing tests that the hash and the
  predicate agree. Use golden local/remote pairs and a shared agreement
  table.
- 🟡 major / high — *Phases 1, 4* — There are no tests for Jira
  frontmatter-kept, unrenderable `--resolve =remote`, unrenderable
  pull-create, or the unrenderable dossier. Use one entry-point × tracker
  table.
- 🟡 major / high — *Phase 7 §6; 8 §2* — There are no `sync_plan.rs` tests
  for `RecordBaseline` or the `PushTitle`/`PullOnly` override, and no
  keyword round trip.
- 🟡 major / medium — *Phase 7 §5* — Recipe-1 writers, realign,
  pending-push recipe-1 markers and recipe-1 `finalise_run` blanking are
  not individually tested.
- 🟡 major / medium — *Phase 8 classify table* — Missing rows: equivalence
  unknown, legacy remote-cosmetic only, not-equivalent recovery rows, and
  frontmatter-only with remote unchanged. Also missing:
  `expect_equivalence_called: false`.
- 🟡 major / medium — *Phase 8 §3* — The `RecordingTracker::rewriting` fake
  is not anchored to the Phase 0 read-back.
- 🔵 minor / medium — *Phase 7 §6* — No test that recipe-1 entries return
  to the stamp-skipped read (`show` call counts).
- 🔵 minor / medium — *Phases 5, 9* — Untested: a `##` inside fenced code,
  a setext heading, a body with no headings, and `work diff` output.
- 🔵 minor / medium — *Phase 7* — The guard's recipe-bumped pass path is
  untested. Make the guard pure and table-test it.
- 🔵 suggestion / low — *Phase 10* — Promotion equivalence is tested only
  for Linear.

### Code Quality

**Summary**: The plan is well layered and test-first, with the body model
split following precedent and equivalence that cannot drift from the hash.
The weak spots are in Phases 4, 7 and 8:
- Jira rules are spread as `Integration` matches.
- `ItemDigests` takes on equivalence and title comparison.
- Several new types use tri-state booleans, bare tuples and near-duplicate
  enums.
- Decision logic is split across classify, decide and plan.

**Strengths**:
- The body model follows the frontmatter split.
- `equivalent` is defined via `canonical()`.
- A named `ProjectedIssue` replaces the private tuple splitter.
- The `LocalBody` and `ApplyError::UnrenderableRemote` types are explicit.
- Phase 10 removes the hand-rolled projection.
- The sketches carry no comments.

**Findings**:
- 🟡 major / high — *Phases 4, 7 §3, 8 §3, 10 §2* — Jira-versus-Linear body
  rules are spread as `Integration` switches. Introduce one `TrackerBody`
  dialect.
- 🟡 major / high — *Phase 7 §2, §4* — The recipe is modelled three ways
  with near-duplicate enums. Use one `DigestRecipe`, `Option<LocalDigest>`
  and names that carry meaning.
- 🟡 major / high — *Phase 8 §2; Phase 7 §2* — The `ItemDigests` port
  becomes a grab-bag with `Option<bool>`. Use `BodyEquivalence` and
  `TitleComparison` enums, and consider a separate port.
- 🟡 major / medium — *Phase 8 §2* — Decision logic is split across
  classify, decide and a plan-level override. Feed the equivalence outcome
  into `decide`.
- 🔵 minor / high — *Phase 8 §2* — `LocalChange` and `PushTitle` names
  don't match what they model.
- 🔵 minor / high — *Phase 7 §5* — `BaselineDigests::of` returns a bare
  tuple and takes an argument clump.
- 🔵 minor / high — *Phase 4 §2* — Unrenderable detection by placeholder
  string.
- 🔵 minor / medium — *Phases 7 §3, 8 §3* — `digest.rs` and
  `LazyItemDigests` accrete mixed responsibilities. Group them by recipe
  module.
- 🔵 minor / medium — *Phases 8 §3, 10 §2* — Promotion reuses
  `bodies_equivalent` from a file-backed object.
- 🔵 suggestion / medium — *Phase 5 §3* — Use named
  `TableRow`/`TableCell`/`TaskMarker` types.
- 🔵 suggestion / medium — *Phase 9 §2* — The new `Document` struct
  collides with existing names.
- 🔵 suggestion / medium — *Phase 3 §4* — The undecided fate of
  `ADF_UNSUPPORTED` risks leaving dead code.

### Compatibility

**Summary**: The plan treats format stability seriously: legacy
byte-identity, legacy markers, read-only preview, an exact parser pin and
the snapshot leak check. The weak point is cross-version compatibility of
the committed `last-sync.json`. Smaller gaps cover unknown-recipe entries,
the recipe of in-flight markers, the exit-code oracle and the
`UnrenderableRemote` mapping.

**Strengths**:
- Legacy entries render byte-identically, and recipe-1 entries keep the
  shared key names.
- Phase 1 defers the recipe change, so no release mixes recipes.
- The exact parser pin comes with a lock guard.
- No `pulldown_cmark` type reaches the pinned APIs.
- ADF divergences are listed explicitly.

**Findings**:
- 🟡 major / high — *Migration Notes; Phase 7 §4* — An older binary pushes,
  not conflicts, on a recipe-1 entry whose stamp is unchanged. Dual-write
  the legacy keys, add a test, and correct the note.
- 🟡 major / high — *Migration Notes; Phase 7 §4* — Any write by an older
  binary downgrades every recipe-1 entry into a mislabelled legacy entry.
  Dual-write, or detect the shape.
- 🟡 major / medium — *Phase 7 §1* — Discarding unknown-recipe entries
  destroys a newer binary's baseline. Keep the entry as opaque JSON and
  classify it `Indeterminate`.
- 🟡 major / medium — *Phase 4 §3* — `UnrenderableRemote` has no failure
  class, exit code or token. Use a `75`-style rejected class with an
  `unrenderable` token, pinned in the golden and the skill.
- 🔵 minor / high — *Phase 3 §4* — Retiring 41 breaks the frozen
  `exit_codes_parity` contract (78 rows). Keep the constant reserved.
- 🔵 minor / medium — *Phase 7 §5* — In-flight promotion markers carry
  remote hashes of unrecorded recipe. Record the recipe or bump
  `PROMOTION_SCHEMA` to 3.
- 🔵 minor / medium — *Phase 7 golden digests* — Jira digests depend on
  the `adf` converter, but there is no golden case for them.
- 🔵 suggestion / medium — *Phase 2* — Pinning `adf` freezes its
  internal modules. Pin only the three entry points.

### Safety

**Summary**: The plan has careful protections: read-only preview,
pull refusal for unrenderable ADF, the parser pin and golden digests, and a
file-free migration. But rule 3's `Unknown`-as-changed and the unspecified
legacy mapping can turn recovery rows into overwrites, the empty-body pull
hazard survives, `PushTitle` can revert remote titles, and unknown-recipe
entries are dropped on write.

**Strengths**:
- `--preview` returns before `ItemApplier`, and byte-identity tests pin it.
- Unrenderable Jira ADF fails the item with no write.
- The parser pin and golden digests stop silent mass reclassification.
- `RecordBaseline` changes no files.
- Existing protections are kept: dirty routing, max-pulls and max-pushes
  bounds, and `finalise_run` blanking.

**Findings**:
- 🟡 major / high — *Phase 8 §2 rule 3* — `Unknown`-as-changed turns
  not-equivalent recovery rows into overwrites. A one-sided `Unknown`
  should conflict, and an `Unknown` local side should force the `show`.
- 🟡 major / high — *Phases 1/4* — A pull writes an empty body when the
  remote body was never fetched (`run.rs:742`), and the local read falls
  back to empty. Fail the item on `None`.
- 🟡 major / medium — *Phase 8 §2* — The unspecified legacy local-change
  mapping could let a pull overwrite real local body edits. Map it to
  `Body` in rule 3.
- 🟡 major / medium — *Phase 8 §2, §3* — `PushTitle` can silently revert
  remote title edits, especially during migration and recovery. Restrict it
  to a recipe-1 frontmatter change and report it.
- 🟡 major / medium — *Phase 7 §1, §4* — Unknown-recipe entries are
  discarded and then deleted on write. Keep them opaque.
- 🔵 minor / high — *Migration Notes* — The older-binary fail-safe claim is
  inaccurate.
- 🔵 minor / medium — *Phase 7 §6* — `RecordBaseline` must not run when the
  forced `show` failed.
- 🔵 suggestion / low — *Phase 4* — Pushes can still overwrite unrenderable
  Jira content.
- 🔵 suggestion / low — *Phase 0* — Allowlist the fixture fields and
  confine the scratch issue.

### Standards

**Summary**: The plan follows the documented conventions closely:
- The `adf` registration covers the README checklist.
- The public-API pins match precedent.
- Naming matches existing code.

Several verification commands do not run as written: wrong package names,
a missing mise task, and credential-gated contract binaries. Two
enforcement mechanisms are also missed: the `deny.toml` wrapper ban and
the frozen exit-code oracle.

**Strengths**:
- Phase 2 registration matches every point of "Registering a library
  crate".
- The `adf` pin follows the technical-library precedent.
- All other mise tasks cited exist.
- The `record-baseline` keyword matches the kebab-case convention.
- `DELIBERATE_DIVERGENCES` follows the count-pinned `ALLOWLIST` pattern.
- The sketches have no comments and use descriptive test names.

**Findings**:
- 🟡 major / high — *Phases 1, 4, 7, 8, 9* — Package names `work-cli` and
  `migrate-cli` do not exist; use `accelerator-work` and
  `accelerator-migrate`.
- 🟡 major / high — *Phases 2, 3, 8* — `cargo test -p jira-client` and
  `-p tracker-test-support` run the credential-gated `contract` binary.
  Use `cargo nextest run`.
- 🟡 major / high — *Phase 3* — `mise run test:integration:characterisation`
  is not a task.
- 🟡 major / high — *Phase 3 §4* — Retiring exit code 41 breaks the frozen
  oracle. Keep it reserved, and list the `adf_inventory.rs` and
  `adf-node-types.txt` edits.
- 🟡 major / medium — *Phase 5* — `pulldown-cmark` is not confined to
  `document` with a `deny.toml` wrapper ban, and the exact-pin rationale
  comment is missing.
- 🟡 major / medium — *Phase 2 §1* — The `cli/adf/src/adf/` layout stutters
  and does not resolve from `lib.rs`. Flatten it and update
  `adf_inventory.rs`.
- 🟡 major / high — *Phase 2* — The probe pair for the new pup rule is never
  run by the phase's checks. Add `mise run test:integration:pup`.
- 🔵 minor / medium — *Phase 7 §6* — The `record-baseline` report keyword is
  not reflected in the skill, and its golden is hedged.
- 🔵 minor / high — *Phases 4, 8* — The opt-in live `tracker-contract` lane
  is listed as an automated gate.
- 🔵 minor / medium — *Phases 0, 7* — The done-bar is inconsistent, and the
  pending-push test location is unresolved.

## Re-Review (Pass 2) — 2026-10-11

**Verdict:** REVISE

### Previously Identified Issues

- 🔴 **Correctness**: Unknown-as-changed turns not-equivalent recovery rows into overwrites — Resolved
- 🟡 **Correctness/Safety**: A pull writes an empty body when the remote body was never fetched — Resolved
- 🟡 **Correctness**: A frontmatter-only edit is Synced when the stamp skips the show — Resolved
- 🟡 **Correctness/Safety**: PushTitle can revert remote title edits — Partially resolved (same-run case fixed; see the two-run retitle issue below)
- 🟡 **Safety**: The legacy local-change mapping could let a pull overwrite local body edits — Resolved
- 🟡 **Compatibility/Correctness**: Older binaries push recipe-1 items and downgrade entries — Resolved
- 🟡 **Compatibility/Safety**: Unknown-recipe entries are discarded and then deleted on write — Resolved
- 🟡 **Compatibility**: `UnrenderableRemote` has no failure class, exit code or report token — Resolved
- 🟡 **Architecture/Code Quality**: Jira body rules spread across the tracker-agnostic sync adapter — Resolved
- 🟡 **Correctness/Architecture**: Unrenderable detection scrapes placeholder text — Resolved
- 🟡 **Architecture/Compatibility**: Recipe-1 digests depend on three crates, but only the parser is guarded — Resolved
- 🟡 **Code Quality**: The recipe is modelled three ways — Partially resolved (`Fingerprints`/`FingerprintSet` still duplicate four facts)
- 🟡 **Code Quality**: The `ItemDigests` port is a grab-bag with `Option<bool>` — Resolved
- 🟡 **Code Quality**: Decision logic is split across classify, decide and plan — Resolved
- 🟡 **Code Quality**: `digest.rs`/`LazyItemDigests` accrete responsibilities — Partially resolved
- 🟡 **Test Coverage**: New ADF fixtures break the 57-case manifest — Resolved
- 🟡 **Test Coverage**: Characterisation tests not listed for inversion — Resolved
- 🟡 **Test Coverage**: No fixed-point round-trip test — Partially resolved (added, but cannot pass over the frozen corpus)
- 🟡 **Test Coverage**: The Jira pull test is its own oracle — Resolved
- 🟡 **Test Coverage**: Pull entry-point coverage gaps — Resolved
- 🟡 **Test Coverage**: Classify table omits recovery and legacy rows — Resolved
- 🟡 **Safety/Correctness**: `record_baseline` reads current bodies at apply time — Resolved
- 🟡 **Standards**: Package names, `cargo test` contract binaries, a missing mise task — Resolved
- 🟡 **Standards**: `pulldown-cmark` not confined to `document` — Resolved
- 🔵 **Correctness/Test Coverage**: Section pairing, Jira dossier comparison, split edge cases, `work diff` — Resolved (two tests live at the wrong layer; see below)
- 🔵 **Standards**: `record-baseline` report keyword — Resolved
- 🔵 **Correctness**: Code-span indentation row needs list-item context — Resolved
- 🔵 **Test Coverage**: Promotion equivalence tested only for Linear — Resolved
- 🔵 **Code Quality**: Named `TableRow`/`TableCell`/`TaskMarker`; `Document` name collision — Resolved
- 🔵 **Safety**: Pushes can overwrite unrenderable Jira content — Resolved
- 🔵 **Safety**: Phase 0 fixture allowlist and scratch-issue deletion — Resolved

### New Issues Introduced

#### Major

- 🟡 **Correctness/Safety/Architecture**: `SendFields` under `--pull-only` records a baseline that absorbs the unpushed local title — the next bidirectional run reads every fact `Unchanged`; contradicts the "pending change the run does not act on stays untouched" rule. Map to `Noop`.
- 🟡 **Correctness/Safety**: A title-only conflict clears itself on the next run — `finalise_run` blanks the local hashes, rule 2 sees `Unknown` plus equivalent bodies and records without consulting `SyncedFields`. Rule 2 must conflict when synced fields differ.
- 🟡 **Safety**: A remote retitle absorbed by `RecordOnly` is reverted by a later unrelated frontmatter edit — `SendFields` keys on the whole frontmatter hash, not on the local synced fields moving.
- 🟡 **Correctness**: Rule 4 drops frontmatter-only edits over persistently non-equivalent bodies (Linear meaning rewrites, rejected Jira bodies) — they classify `Synced` and are never pushed.
- 🟡 **Correctness**: Rule 1 omits Phase 7's `RecordOnly` for unchanged `Whole` entries, so legacy entries stop migrating from Phase 8 onwards.
- 🟡 **Code Quality**: `Entry` is described as both a struct holding `Option<FingerprintSet>` and an enum with `Unsupported` — make it a sum type mirroring the domain.
- 🟡 **Test Coverage/Correctness**: The fixed-point test over the frozen corpus cannot pass — ordered-list order, abort cases and unescaped inline syntax are retained quirks; the escape set is too narrow.

#### Minor

- 🔵 **Architecture/Code Quality/Correctness**: `RecordBaseline` is now planned for `LocallyModified` items, breaking Phase 7's "only for `Synced`" report contract.
- 🔵 **Architecture/Correctness/Test Coverage/Standards**: A stray classify row sits outside the table with `—` contradicting rule 3; Phase 8 §3 files `BodyComparison::equivalence` under `digest.rs`.
- 🔵 **Architecture/Code Quality**: The synced-field set is encoded in three places just before 0290 extends it.
- 🔵 **Architecture**: `BodyComparison` depends on the whole `RemoteTracker`, which `work diff` does not have.
- 🔵 **Code Quality/Test Coverage**: `Classification` admits meaningless state/reconciliation pairs; `SyncedFields::Unknown` has no outcome or row.
- 🔵 **Code Quality**: `Fingerprints::Split` and `FingerprintSet` duplicate four facts with a `""` sentinel; three error names for one concept; `BaselineDigests::of` returns an unnamed pairing.
- 🔵 **Compatibility**: "Every recipe renders keys alphabetically" contradicts byte-identical legacy rendering.
- 🔵 **Compatibility/Standards**: The skill's "detail-token table" does not exist; `unsupported-recipe` and `failed / synced / retryable` are undocumented in the skill.
- 🔵 **Compatibility**: 0331 removing the dual-written keys without a recipe bump reintroduces the old-binary push hazard.
- 🔵 **Safety**: A title-only `SendFields` re-sends the body, stripping Jira underline.
- 🔵 **Test Coverage**: Fenced-code and setext section tests sit on hand-built models; Phase 1's pull-create row only exercises a fake author.
- 🔵 **Standards**: `adf`'s pin exposes `serde_json::Value` unacknowledged; the crate-dependencies lint cannot verify the dev-only `adf` edge.
- 🔵 **Compatibility/Standards** (suggestions): unrenderable dossier header status; `--render-adf` release note; malformed `digest_recipe` values; `public-api:update` list omits `adf` and `tracker`.

### Assessment

Every critical finding and all but four majors from pass 1 are resolved, and the boundary work (tracker body dialect, `BodyComparison`, dual-written baseline, opaque unknown recipes) drew praise from every lens. The remaining risk is concentrated in Phase 8's new title reconciliation: four majors describe paths where a baseline write absorbs or reverts a title difference across runs, and one where legacy migration stops. These are fixable inside the decision table with a local-fields fingerprint, a stricter rule 2, `Noop` under `--pull-only`, and explicit `Whole` handling in rule 1, each pinned by a two-run test. The `Entry` shape and the fixed-point test scope need a decision each; the minors are mostly consistency edits.


## Re-Review (Pass 3) — 2026-10-11

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness/Safety/Architecture**: `SendFields` under `--pull-only` absorbs the unpushed local title — Resolved
- 🟡 **Correctness/Safety**: A title-only conflict clears itself on the next run — Resolved
- 🟡 **Safety**: A recorded remote retitle is reverted by a later unrelated frontmatter edit — Partially resolved (fixed in the equivalent branch for recipe-1 entries; still reachable through rule 4 and through `Whole` entries)
- 🟡 **Correctness**: Rule 4 drops frontmatter-only edits over non-equivalent bodies — Resolved (the new predicate introduces the absorbed-local-title issue below)
- 🟡 **Correctness**: Rule 1 omits `RecordOnly` for unchanged `Whole` entries — Resolved
- 🟡 **Code Quality**: `Entry` described as both struct and enum — Resolved
- 🟡 **Test Coverage/Correctness**: The fixed-point test cannot pass over the frozen corpus — Partially resolved (ordered start and escaping fixed; placeholder, level-9 heading, empty-text and adjacent-mark cases remain)
- 🔵 **Architecture/Code Quality/Correctness**: `RecordBaseline` for `LocallyModified` breaks the report contract — Resolved
- 🔵 **Architecture/Correctness/Test Coverage/Standards**: Stray classify row; misfiled `equivalence` — Resolved
- 🔵 **Architecture/Code Quality**: Synced-field set encoded in three places — Resolved
- 🔵 **Architecture**: `BodyComparison` depends on the whole `RemoteTracker` — Resolved
- 🔵 **Code Quality/Test Coverage**: Meaningless `Classification` pairs; `SyncedFields::Unknown` has no outcome — Partially resolved (`Synced(SendFields)` still representable, pinned only by a test)
- 🔵 **Code Quality**: `Fingerprints`/`FingerprintSet` duplication, three error names, unnamed pairing — Partially resolved (legacy pair still declared three times)
- 🔵 **Code Quality**: `digest.rs`/`LazyItemDigests` accrete responsibilities — Partially resolved (moved into `BodyComparison`; see below)
- 🔵 **Compatibility**: Alphabetical keys contradict byte-identical legacy rendering — Resolved
- 🔵 **Compatibility/Standards**: Phantom detail-token table; undocumented `unsupported-recipe` and `failed / synced` rows — Partially resolved (edits point at a "report description" and `needs-upgrade` group that do not exist)
- 🔵 **Compatibility**: 0331 removing dual-written keys without a recipe bump — Resolved
- 🔵 **Safety**: Title-only `SendFields` re-sends the body — Resolved
- 🔵 **Test Coverage**: Section tests on hand-built models; fake-author pull-create — Resolved
- 🔵 **Standards**: `adf` pin exposes `serde_json::Value`; dev-only lint edge — Partially resolved (the real new edge is `tracker-test-support → adf`; see below)
- 🔵 **Compatibility/Standards** (suggestions): dossier header status, `--render-adf` release note, malformed `digest_recipe`, `public-api:update` list — Resolved

### New Issues Introduced

#### Major

- 🟡 **Correctness/Safety**: Rule 4 ignores remote fields, so a `ByContent` push over non-equivalent bodies reverts a remote retitle — formerly a conflict under the whole-content `remote_hash`; the "remote title changed, not equivalent → synced/by content" row also leaves `remote_fields_hash` stale for the next local edit.
- 🟡 **Correctness/Safety**: A local title edit facing a remote body edit is pulled and absorbed — rule 4's local-change predicate ignores local fields, so the pull's baseline records the new local title and the title is never sent.
- 🟡 **Test Coverage/Correctness**: The fixed-point test still cannot pass — the placeholder cases, `render-headings` level 9, `render-text-without-text-field` and `render-marks-pipeline` adjacent marks are not fixed points; escaping inside code marks and code blocks is unspecified.
- 🟡 **Architecture/Standards**: `RecordingTracker::jira` imports `adf`, which `tracker_test_support_imports_only_permitted` rejects and `tracker-test-support/Cargo.toml` lacks; the double also duplicates the Jira dialect rather than reusing `JiraClient`'s.
- 🟡 **Architecture**: Promotion's `IntendedBaseline::next_sync` remains a second classifier comparing one digest, not reconciled with recipe-1 classification or schema-2 legacy resumption.
- 🟡 **Compatibility**: The advertised `--resolve <id>=local` remedy is inert for unrenderable pulls and refused pushes — `plan` applies resolutions only to `Prompt` items, and a resolved push is indistinguishable from a planned one.
- 🟡 **Standards**: Sync contract changes (exit 71/70/75, schema-3 markers, recipe-1 rewrite, `unsupported-recipe`, `record-baseline`, `work diff` omissions) have no CHANGELOG entries.
- 🟡 **Code Quality**: Baseline vocabulary differs per layer (`Whole`/`Split` vs `Legacy`/`Fingerprinted`, `Fingerprints` vs `FingerprintSet`) and the legacy pair is declared three times.
- 🟡 **Code Quality**: `BodyComparison` has outgrown its name — it builds the whole `BaselineDigests`, including frontmatter, fields and legacy hashes.

#### Minor

- 🔵 **Safety/Correctness**: `Whole` entries with an absorbed remote retitle still send the stale title on an unrelated frontmatter edit, contradicting the amendment.
- 🔵 **Safety**: Legacy migration baselines files already polluted by earlier pulls (title line, raw ADF JSON) without flagging them.
- 🔵 **Correctness**: `RecordOnly` for an unchanged legacy entry has no defined path when the forced `show` fails.
- 🔵 **Correctness**: `local_fields` must decode the title as `create_request_fields::read` does, or quoted titles never compare `Same`.
- 🔵 **Code Quality**: `Synced(SendFields)` is representable; `LegacyEntry` holds recipe-independent stamps; the escape set duplicates the tokeniser's syntax set; `Different`/`Differ` and `DossierUnrenderable` naming.
- 🔵 **Test Coverage**: Retitle and record-only outcomes lack a settling follow-up run; the `""`↔`None` codec mapping, `digest_recipe: 0` and `remove` on `Unsupported` are unpinned; the skill's unrenderable prompt has no `rg` check.
- 🔵 **Standards**: The reduced `[local/skip]` prompt is not pinned like the existing conflict prompt; the `adf` denial sits in `work_adapters_is_zero_spawn`; new `ApplyError` variants use `id` not `item_id`; the `corpus-adapters` lint criterion sits in Phase 4 rather than 7; `baseline_digests` lives apart from `BodyComparison`.
- 🔵 **Compatibility/Architecture** (suggestions): `MarkerObservingTracker` missing from the `retitle` implementer list; the `adf:` digest prefix leaks Jira into `BodyComparison`; the unrenderable dossier keeps no copy of the content `=local` replaces.

### Assessment

Every pass-2 major in the equivalent-body branch is resolved, and all seven lenses praised the `BodyDialect` port, `SyncedTrackerFields`, the `Entry` sum type and the title-only `retitle`. The remaining correctness risk has moved to rule 4: the non-equivalent branch still reasons over bodies alone, so both a remote retitle and a local title edit can be lost where today's whole-content hashes report a conflict; folding the fields facts into rule 4's change predicates closes both. The other majors are an unworkable `--resolve =local` remedy for non-conflict items, an un-passable fixed-point test, a pup boundary the Phase 4 test double breaks, promotion's separate prediction, missing CHANGELOG entries, and two naming consolidations. Most minors are mechanical.

## Re-Review (Pass 4) — 2026-10-11

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Correctness/Safety**: Rule 4 ignores remote fields, so a `ByContent` push reverts a remote retitle — Resolved
- 🟡 **Correctness/Safety**: A local title edit facing a remote body edit is pulled and absorbed — Resolved
- 🟡 **Test Coverage/Correctness**: The fixed-point test cannot pass — Partially resolved (`render-marks-pipeline` strong-over-code is still not a fixed point; the oracle differential's divergence list is incomplete)
- 🟡 **Architecture/Standards**: `RecordingTracker::jira` imports `adf` — Resolved
- 🟡 **Architecture**: Promotion's `next_sync` is a second classifier — Resolved
- 🟡 **Compatibility**: `--resolve <id>=local` is inert for unrenderable items — Resolved (the new `Action::Overwrite` escapes the push filters; see below)
- 🟡 **Standards**: Sync contract changes lack CHANGELOG entries — Resolved (subsections unspecified; see below)
- 🟡 **Code Quality**: Baseline vocabulary differs per layer — Resolved
- 🟡 **Code Quality**: `BodyComparison` has outgrown its name — Resolved
- 🔵 **Safety/Correctness**: Legacy entries with an absorbed remote retitle send the stale title — Partially resolved (guarded in rule 3 only; rule 4 still sends it)
- 🔵 **Safety**: Legacy migration baselines polluted files — Partially resolved (an unchanged polluted file is a conflict; an edited one is pushed through rule 4)
- 🔵 **Correctness**: Failed forced `show` for an unchanged legacy entry — Resolved
- 🔵 **Correctness**: `local_fields` title decoding — Resolved
- 🔵 **Code Quality**: `Synced(SendFields)` representable; stamps; escape-set duplication; naming — Resolved
- 🔵 **Test Coverage**: Settling follow-up; codec edge cases; unrenderable prompt `rg` check — Partially resolved (the settling test contradicts the `--pull-only` deferral test)
- 🔵 **Standards**: Pinned reduced prompt; separate pup rule; `item_id`; lint criterion phase; `baseline_digests` placement — Resolved
- 🔵 **Compatibility/Architecture** (suggestions): `MarkerObservingTracker`; `adf:` prefix — Resolved; raw ADF copy beside the unrenderable dossier — declined deliberately

### New Issues Introduced

#### Major

- 🟡 **Safety/Correctness/Architecture**: The legacy diverged-title guard sits only in rule 3 — a legacy item retitled remotely and pulled by today's binary carries the projected title line, so its bodies are non-equivalent; an unrelated `status:` edit then takes rule 4 as `ByContent{Send}`, reverting the remote title and pushing the polluted body (or raw ADF JSON).
- 🟡 **Correctness/Test Coverage**: A frontmatter-only local edit facing a remote retitle over non-equivalent bodies reads as `RemotelyModified` — the pull overwrites a local body neither side edited; no classify row pins the case.
- 🟡 **Safety/Correctness**: `RemoteUnrenderable → Conflict` catches remote-only, title-only and frontmatter-only changes — the only non-skip remedy is the destructive `=local` overwrite, and `finalise_run` blanking keeps the conflict permanent; `retitle` could deliver a title change without touching the description.
- 🟡 **Compatibility/Architecture**: The legacy `Equivalent`-only gate over-applies on upgrade — untouched Jira items with smart links, mentions or images (`RemoteUnrenderable`) and untouched Linear items with meaning-changing rewrites (`run.rs`-style autolinks) become permanent conflicts although they are `synced` today.
- 🟡 **Compatibility**: `Action::Overwrite` falls outside the `== Action::Push` filters — `SyncPlan::push_count` (`--max-pushes`) and preview's `validate_pushes` ignore it, so a destructive overwrite escapes the push budget and the payload check.
- 🟡 **Code Quality**: `Action::Push` cannot distinguish a title-only `SendFields` push from a body push — the applier dispatches on `Action` alone, yet must call `retitle` for one and `update` for the other.
- 🟡 **Test Coverage**: The fixed-point test still fails on `render-marks-pipeline` — strong over code renders as ``[**`all`**](…)``, `parse_inlines` does not recurse into strong, and the second render escapes the backticks.
- 🟡 **Test Coverage**: The oracle differential's `DELIBERATE_DIVERGENCES` criteria omit four render changes (`render-listitem-second-child-dropped`, `render-listitem-nested-list-first-child`, `render-marks-silently-ignored`, `render-text-without-text-field`).
- 🟡 **Test Coverage**: Phase 4's Jira push-refusal and `=local` overwrite tests are filed in the domain `work` crate, which has no ADF, Jira dialect or applier — the applier's `Overwrite` behaviour has no failing test.

#### Minor

- 🔵 **Correctness/Test Coverage**: `every_settling_outcome_syncs_as_synced_on_the_next_run` lists the `--pull-only` deferral, contradicting `a_pull_only_run_leaves_a_pending_title_change_for_the_next_run`; "no tracker call" also forbids the bulk read.
- 🔵 **Compatibility/Architecture**: "Phases 7 and 8 ship in the same release" has no mechanism, and every push to `main` cuts a prerelease; a Phase-7-only build permanently baselines polluted legacy files as `Split`.
- 🔵 **Standards/Test Coverage**: Nextest commands omit `-p linear-client` (Phase 4) and `-p tracker -p jira-client -p linear-client` (Phase 8).
- 🔵 **Standards**: Phase 9 changes `work::section_diff::differing_sections` without a `public-api:update` step; CHANGELOG entries name no `[Unreleased]` subsection; Phases 7 and 8 skill text has no `rg` check.
- 🔵 **Code Quality**: `BodyComparison`'s stated surface lacks the operations Phases 9 and 10 call; `work-cli/src/list.rs` (a `LazyItemDigests` caller) is unmentioned; `UnrenderableRemote`/`RemoteUnrenderable` word order and two meanings of `reason`; `BaselineDigests`/`BaselineFingerprints` near-synonyms; stale `fingerprint_set` test name.
- 🔵 **Architecture**: Phase 2's stated rationale for the `adf` crate (`tracker-test-support` rendering ADF) no longer holds.
- 🔵 **Code Quality/Architecture** (suggestions): number the unrenderable check as its own rule; have the fetch skip-probe call the domain's local-change computation rather than re-implement it.

### Assessment

The pass-3 field-aware rule 4, the Jira dialect injection, the single classifier for promotion and the unified vocabulary all hold, and the lenses praised each. The remaining correctness risk is now concentrated at two edges: legacy entries and unrenderable remotes. In both, a rule written for one branch (the legacy title guard in rule 3; the unrenderable conflict in rule 1) is either too narrow, letting rule 4 push stale titles and polluted bodies, or too broad, turning harmless metadata changes and untouched upgraded items into permanent conflicts whose only remedy is destructive. The other majors are mechanical: `Overwrite` must count as a push, the applier needs a distinct title-only action, two Phase 3 test specifications need completing, and two Phase 4 tests belong in `work-adapters`.

## Re-Review (Pass 5) — 2026-10-12

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Safety/Correctness/Architecture**: Legacy diverged-title guard only in rule 3 — Resolved
- 🟡 **Correctness/Test Coverage**: Frontmatter-only edit facing a remote retitle over differing bodies pulls — Resolved
- 🟡 **Safety/Correctness**: `RemoteUnrenderable → Conflict` too broad — Resolved for `Split`; partially for `Legacy` (see below)
- 🟡 **Compatibility/Architecture**: Legacy `Equivalent`-only gate over-applies on upgrade — Resolved
- 🟡 **Compatibility**: `Action::Overwrite` escapes the push filters — Resolved
- 🟡 **Code Quality**: `Action::Push` cannot distinguish a title-only push — Resolved
- 🟡 **Test Coverage**: Fixed-point test fails on `render-marks-pipeline` — Resolved
- 🟡 **Test Coverage**: Oracle differential's divergence list incomplete — Partially resolved (`render-block-placeholders` still missing)
- 🟡 **Test Coverage**: Phase 4 tests misfiled in `work` — Resolved
- 🔵 **Correctness/Test Coverage**: Settling test contradicts `--pull-only` deferral — Resolved
- 🔵 **Compatibility/Architecture**: Phase 7/8 same-release coupling — Partially resolved (the residue gate is undone on the next run; see below)
- 🔵 **Standards/Test Coverage**: Nextest packages; Phase 9 public-api; CHANGELOG subsections; Phase 7/8 `rg` checks — Resolved
- 🔵 **Code Quality**: `BodyComparison` surface; `list.rs`; unrenderable word order; near-synonyms; stale test name — Resolved
- 🔵 **Architecture**: `adf` crate rationale; fetch probe duplication; unrenderable rule placement — Resolved

### New Issues Introduced

#### Major

- 🟡 **Safety/Correctness**: Legacy conflicts over an unrenderable Jira remote offer only `=local` — a residue conflict pushes the title line and raw ADF JSON into Jira, and a legacy metadata-only edit can be delivered only by deleting mentions or media; skipping leaves a permanent conflict.
- 🟡 **Architecture**: Phase 7's residue gate covers only the untouched `Synced` case — `finalise_run` blanks the conflict's local hash, and Phase 7's absent-hash rule then plans a push of the polluted body on the next run.
- 🟡 **Standards**: `BodyDialect::is_wire_document` has a default that the Phase 4 delegating implementations (`JiraClient`, `RecordingTracker::with_dialect`) do not forward, so the shipped client never detects ADF residue and the ADF residue test cannot pass.
- 🟡 **Test Coverage**: `DELIBERATE_DIVERGENCES` omits `render-block-placeholders` (blockquote and rule now render) and leaves the same-mark-set run merging in `render-marks-pipeline` unstated.

#### Minor

- 🔵 **Compatibility/Test Coverage**: Preview validates a full title-and-body payload for `Retitle` and title-withheld `Push`; no test for a previewed retitle.
- 🔵 **Compatibility/Standards**: Exit-code, report-row and downgrade changes in Phases 4, 7 and 8 sit outside `### Breaking`.
- 🔵 **Compatibility/Test Coverage**: A Phase-4-only release leaves locally edited Jira items stuck at exit 75 with no `=local`; Phase 8 leaves the applier's push refusal untested.
- 🔵 **Correctness/Safety**: The residue gate fires over equivalent bodies, producing an empty dossier.
- 🔵 **Correctness/Code Quality/Architecture**: Rule 4's header omits `RemoteUnrenderable`; two classify rows say `push` for a retitle; `expect_comparison_called` is unstated for `pull_residue`.
- 🔵 **Architecture/Test Coverage**: `FakeComparison` and residue classify rows sit in Phase 8 although the port lands in Phase 7; no end-to-end Jira unrenderable legacy migration test.
- 🔵 **Code Quality**: `Entry` field names differ across variants with stale prose; legacy exceptions interleaved across three rules; `markdown_equivalence` hides an asymmetric step; `ItemSnapshot` file placement; `PullResidue::AdfDocument` names Jira; `SendFields`/`Retitle` naming.
- 🔵 **Standards**: Alternation `rg` checks pass on one of two edits; a test name says "refused" for a conflict.
- 🔵 **Architecture** (suggestions): `pull_residue` belongs in `BodyComparison`; add the transitional residue surface to 0331's removal scope.

### Assessment

The `Split` classification is now consistent across every row, and the lenses found no remaining data-loss path for recipe-1 entries. What remains sits at the intersection of legacy migration and unrenderable Jira remotes: the residue gate must survive its own conflict, and the dossier must not offer an overwrite that pushes residue or deletes content to deliver metadata. The other majors are a missing delegation and an incomplete divergence list, both mechanical.

## Re-Review (Pass 6) — 2026-10-12

**Verdict:** REVISE

Lenses re-run: correctness, safety, test-coverage (the edits since pass 5 touched legacy migration and the Jira edge cases only).

### Previously Identified Issues

- 🟡 **Safety/Correctness**: Legacy conflicts over an unrenderable remote offer only `=local` — Partially resolved (the overwrite is refused from Phase 7, but the conflict is permanent; see below)
- 🟡 **Architecture**: Phase 7's residue gate undone on the next run — Resolved
- 🟡 **Standards**: `is_wire_document` not forwarded — Resolved
- 🟡 **Test Coverage**: `DELIBERATE_DIVERGENCES` incomplete — Resolved (every frozen render case checked)
- 🔵 **Compatibility/Test Coverage**: Preview payload validation — Partially resolved (tests vacuous through `RecordingTracker`)
- 🔵 **Compatibility/Test Coverage**: Phase-4-only release; untested applier guard — Resolved
- 🔵 **Correctness/Safety**: Residue gate over equivalent bodies — Resolved from Phase 8 (transitional in a Phase-7-only release)
- 🔵 **Correctness**: Rule 4 header; `push` rows; `expect_comparison_called` — Resolved
- 🔵 **Architecture/Test Coverage**: `FakeComparison` placement; Jira end-to-end migration — Resolved

### New Issues Introduced

#### Major

- 🟡 **Safety/Correctness**: The `legacy-unrenderable` remedies do not converge — rendering the body with placeholders leaves rule 2 at `Conflict` (the blanked hash plus `RemoteUnrenderable`); reverting a metadata edit cannot restore the blanked hash; delete-and-pull-create deletes the local file and then the pull-create is refused. Every pre-plan Jira pull with a mention, panel or media becomes a permanent conflict.
- 🟡 **Safety/Correctness**: Residue protections are keyed on `Entry::Legacy` and land in Phase 7 — an entry-less residue item in any release, and every item in a Phase-4-to-6 release, still reaches `=local` over a `remote-unrenderable` dossier that shows no body, pushing the title line and ADF JSON into Jira.
- 🟡 **Test Coverage/Correctness**: The Phase 7 legacy guard flips Phase 4's `remote-unrenderable` dossier and `=local` overwrite tests, which seed legacy-shaped entries; the plan names no recipe-1 seeding helper.

#### Minor

- 🔵 **Correctness**: Preview cannot see the applier-only unrenderable refusals, so it reports a validated `push` the run refuses.
- 🔵 **Test Coverage**: The residue gate's changed-state branches have no classify rows; the Phase 8 absent-hash residue row states no equivalence.
- 🔵 **Test Coverage**: The previewed retitle and withheld-push tests are vacuous through `RecordingTracker`; no client-level `validate_update` `Option` tests.
- 🔵 **Test Coverage**: The `| a | b` reading is undecided, so `assemble-table-guard-narrow` may need a divergence entry.
- 🔵 **Test Coverage**: A realistic legacy Jira file hits `ProjectedTitleLine`, so `TrackerWireDocument` is not exercised end to end.

### Assessment

The `Split` path and the Phase 3 corpus are now settled; correctness walked every classify row without a contradiction. The remaining majors are one problem seen from three sides: residue from pre-plan pulls over unrenderable Jira descriptions needs a protection that is keyed on the residue itself rather than the entry shape, lands with the first phase that can offer `=local`, and gives the user a remedy that actually settles the item.

## Re-Review (Pass 7) — 2026-10-12

**Verdict:** REVISE

Lenses re-run: correctness, safety, test-coverage.

### Previously Identified Issues

- 🟡 **Safety/Correctness**: `legacy-unrenderable` remedies do not converge — Partially resolved (the Jira-edit remedy converges; revert-sync-reapply does not in a Phase-7 release or when the remote changed)
- 🟡 **Safety/Correctness**: Residue protections keyed on `Entry::Legacy` and landing in Phase 7 — Partially resolved (body-keyed from Phase 4, but blind when the stamp skips the `show`, and evaded by a stale title line)
- 🟡 **Test Coverage/Correctness**: Phase 7 guard flips Phase 4 tests — Resolved (`split_entry`)
- 🔵 **Correctness**: Preview cannot see applier refusals — Resolved (shared `send_refusal`)
- 🔵 **Test Coverage**: Residue changed-state rows; vacuous preview tests; `| a | b`; `TrackerWireDocument` end to end — Resolved

### New Issues Introduced

#### Major

- 🟡 **Correctness**: `send_refusal` is blind when the stamp skips the `show` (Phases 4–7) — a local edit over an unchanged Jira remote sends residue or deletes unrenderable content.
- 🟡 **Correctness**: A projected title line made stale by a remote retitle evades detection, so `=local` sends the title line and ADF JSON.
- 🟡 **Safety/Correctness**: `ProjectedTitleLine` flags descriptions that legitimately open with their title, refusing every later push on a `Split` item; `=remote` is inert outside a conflict.
- 🟡 **Safety/Correctness**: The `pull-residue` remedy (`=remote`) does nothing for non-conflict items and discards local edits for conflicts; the edit-preserving route is never named.
- 🟡 **Safety/Correctness**: `VerbatimRemote` with an absent local hash records over an unsent local title edit after one skipped conflict.
- 🟡 **Correctness/Test Coverage**: The revert-sync-reapply remedy cannot settle in a Phase-7 release (the reapplied edit is a refused `Push`), so `each_named_legacy_unrenderable_remedy_settles_the_item` cannot pass there.

#### Minor

- 🔵 **Test Coverage/Correctness**: Residue test fixtures do not say their ADF differs from the remote, which now decides `VerbatimRemote`.
- 🔵 **Test Coverage**: `send_refusal`'s entry-shape branch has no unit rows; `VerbatimRemote` lacks absent-hash and remote-changed rows; `pull-residue` detail and exit 75 unpinned at the report layer.
- 🔵 **Correctness**: Untouched Linear pulls with a title line conflict, contrary to the Phase 7 CHANGELOG and manual check.
- 🔵 **Correctness**: The `local-pull-residue` dossier names only the lossy Jira edit when revert-sync-reapply would settle a verbatim pull losslessly.

### Assessment

The `Split` path, the Phase 3 corpus and the classify table remain sound. Every new finding sits in pre-plan residue handling, and most trace to the title-line detector: it cannot tell a projected title line from a legitimate first line, misses a stale one, and drags Linear files into a mechanism only Jira's raw ADF needs. Narrowing residue to the tracker's wire document, forcing the remote read whenever a send is planned, and moving the legacy-unrenderable remedy to the phase that can deliver it would remove most of the remaining surface.

## Re-Review (Pass 8) — 2026-10-12

**Verdict:** REVISE

Lenses re-run: correctness, safety, test-coverage.

### Previously Identified Issues

- 🟡 **Correctness**: `send_refusal` blind when the stamp skips the `show` — Resolved (blanked-hash wording gap, minor)
- 🟡 **Correctness**: Stale title line evades detection — Resolved
- 🟡 **Safety/Correctness**: `ProjectedTitleLine` false positives — Resolved (the concept is gone; see the `VerbatimRemote` major)
- 🟡 **Safety/Correctness**: `pull-residue` remedy inert or lossy — Resolved
- 🟡 **Safety/Correctness**: `VerbatimRemote` with an absent hash absorbs a local title — Resolved
- 🟡 **Correctness/Test Coverage**: Revert-sync-reapply cannot settle in Phase 7 — Resolved (`legacy-unrenderable` moved to Phase 8)
- 🔵 **Test Coverage**: Residue fixtures; `send_refusal` rows; verbatim rows; `pull-residue` report pin — Resolved
- 🔵 **Correctness**: Linear title-line pulls conflict contrary to the CHANGELOG — Resolved

### New Issues Introduced

#### Major

- 🟡 **Correctness/Safety/Test Coverage**: `VerbatimRemote` is not gated on a wire document, so a Linear body equal to its remote (with or without a title line) reads as residue, refusing ordinary pushes and `=local` overwrites.
- 🟡 **Test Coverage**: `a_local_edit_over_an_unchanged_stamp_still_meets_the_send_refusal` flips to a conflict under the Phase 7 gate and is not on the `split_entry` switch list.
- 🟡 **Test Coverage/Safety/Correctness**: The `local-pull-residue` revert-sync-reapply remedy has no test and cannot settle before Phase 8.

#### Minor

- 🔵 **Test Coverage/Correctness**: Phase 4's forced read has no fetch-level test until Phase 8, and does not say a blanked local hash forces it.
- 🔵 **Test Coverage**: Phase 7's absent-hash and no-entry verbatim rows are not marked as replaced in Phase 8.
- 🔵 **Correctness**: The `legacy-unrenderable` dossier is keyed on rule-4 conflicts while the `Overwrite` refusal is keyed on entry shape.
- 🔵 **Correctness**: Phase 8 places the residue gate "before rule 2" rather than before rule 1.

### Assessment

No new design question arose: every finding is a wording slip or a test placed in the wrong phase, and all were applied to the plan after this pass — `VerbatimRemote` now requires the dialect's wire document, the unchanged-stamp test moves to `split_entry` in Phase 7, the `local-pull-residue` revert remedy and its convergence test move to Phase 8, Phase 4 gains fetch-level tests and covers a blanked hash, the superseded rows are listed, one predicate keys both the dossier and the refusal, and the gate runs before rule 1. The `Split` path, the Phase 3 corpus and the classify table have held for three passes.

## Approval — 2026-10-12

**Verdict:** APPROVE

Every pass-8 finding was applied to the plan; the plan is marked ready for implementation.

---
*Review generated by /accelerator:review-plan*
